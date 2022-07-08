use anyhow::anyhow;
use secrecy::Secret;
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use configurations::{
    generate_session_data,
    password::{verify_password, AuthError},
    session::{SessionData, TypedSession},
    telemetries::spawn_blocking_with_tracing,
    Settings,
};
use domains::models::{
    users::{HashedPassword, RawPassword, User, UserId, UserName},
    EmailAddress,
};
use infrastructures::repositories::users::{PgUserRepository, UserRepositoryError};

#[derive(Debug, thiserror::Error)]
pub enum SignupError {
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    #[error("Eメールアドレスが既に登録されています。")]
    EmailAddressAlreadyExists,
}

#[derive(Debug, Serialize)]
pub struct SignupResult {
    pub id: Uuid,
    pub user_name: String,
    pub email_address: String,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn signup(
    user_name: UserName,
    email_address: EmailAddress,
    password: RawPassword,
    pool: &PgPool,
) -> anyhow::Result<SignupResult, SignupError> {
    // トランザクションを開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;
    // リポジトリを構築
    let repository = PgUserRepository::default();

    // メールアドレスが一致するユーザーが存在しないか確認
    let found = repository
        .get_by_email_address(&email_address, &mut tx)
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;
    if found.is_some() {
        return Err(SignupError::EmailAddressAlreadyExists);
    }

    // ユーザーを登録
    let hashed_password = HashedPassword::new(&password).map_err(SignupError::UnexpectedError)?;
    let user = User::new(
        UserId::default(),
        user_name,
        email_address,
        hashed_password,
        true,
        None,
        None,
        None,
    );
    let user = repository
        .insert(&user, &mut tx)
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;

    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| SignupError::UnexpectedError(e.into()))?;

    Ok(SignupResult {
        id: user.id().value().to_owned(),
        user_name: user.user_name().value().to_owned(),
        email_address: user.email_address().value().to_owned(),
        is_active: user.is_active(),
        created_at: user.created_at().unwrap(),
        updated_at: user.updated_at().unwrap(),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    #[error("Eメールアドレスまたはパスワードが異なります。")]
    InvalidCredentials,
    #[error("ユーザー({0})が無効になっています。")]
    NotActive(Uuid),
}

/// データベースからユーザーを取得して、パスワードを検証する。
///
/// # Arguments
///
/// * `email_address` - Eメールアドレス。
/// * `raw_password` - パスワード。
/// * `tx` - トランザクション。
///
/// # Returns
///
/// * ユーザーインスタンス。
#[tracing::instrument(name = "Validate credentials", skip(raw_password, tx))]
async fn validate_credentials(
    email_address: EmailAddress,
    raw_password: Secret<String>,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<User, LoginError> {
    // Eメールアドレスからユーザーを取得
    let result = PgUserRepository::default()
        .get_by_email_address(&email_address, tx)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if result.is_none() {
        return Err(LoginError::InvalidCredentials);
    }

    // 引数で受け取ったパスワードをハッシュ化した結果が、ユーザーに記録されているハッシュ化パスワードと一致するか確認
    let user = result.unwrap();
    let expected_hashed = user.hashed_password().value().to_owned();
    let result =
        spawn_blocking_with_tracing(move || verify_password(&expected_hashed, &raw_password))
            .await
            .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    if let Err(e) = result {
        return Err(match e {
            AuthError::InvalidCredentials(_) => LoginError::InvalidCredentials,
            AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e),
        });
    }

    Ok(user)
}

/// ユーザーの最終更新日時を更新する。
///
/// # Arguments
///
/// * `user_id` - 最終更新日時を更新するユーザーのユーザーID。
///
/// # Returns
///
/// `()`。
async fn update_last_logged_in(
    user_id: UserId,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), LoginError> {
    PgUserRepository::default()
        .update_last_logged_in(user_id, tx)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    Ok(())
}

/// ログインする。
///
/// ログインを試行して、ログインに成功したら、ユーザーの最終ログイン日時を更新して、Redisにセッションデータ
/// を登録する。
pub async fn login(
    email_address: EmailAddress,
    raw_password: Secret<String>,
    settings: &Settings,
    session: &TypedSession,
    pool: &PgPool,
) -> anyhow::Result<SessionData, LoginError> {
    // トランザクションを開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // データベースからユーザーを取得して、パスワードを検証
    let user = validate_credentials(email_address, raw_password, &mut tx).await?;

    // ユーザーがアクティブでない場合は、エラーを返却が確認
    if !user.is_active() {
        return Err(LoginError::NotActive(user.id().value()));
    }

    // セッションデータを生成
    let Settings { tokens, .. } = settings;
    #[allow(clippy::redundant_closure)]
    let session_data = generate_session_data(user.id().value(), tokens)
        .map_err(|e| LoginError::UnexpectedError(e))?;

    // セッション固定化攻撃に対する対策として、セッションを更新
    session.renew();
    // セッションデータをセッションストアに登録
    session
        .insert(&session_data)
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // ユーザーの最終ログイン日時を更新
    update_last_logged_in(user.id(), &mut tx).await?;

    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;

    // セッションデータを返却
    Ok(session_data)
}

#[derive(Debug, thiserror::Error)]
pub enum ChangePasswordError {
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    #[error("現在のパスワードが間違っています。")]
    IncorrectCurrentPassword,
    #[error("ユーザー({0})が存在しません。")]
    NotFound(Uuid),
}

/// パスワードを変更する。
///
/// パスワードの変更を試行して、パスワードの変更に成功したら、Redisに格納されたセッションデータを削除する。
pub async fn change_password(
    user: &User,
    current_password: RawPassword,
    new_password: RawPassword,
    session: &TypedSession,
    pool: &PgPool,
) -> anyhow::Result<(), ChangePasswordError> {
    // ユーザーの現在のパスワードが一致するか確認
    let expected_hashed = user.hashed_password().value().to_owned();
    let _ = spawn_blocking_with_tracing(move || {
        verify_password(&expected_hashed, current_password.value())
    })
    .await
    .map_err(|e| ChangePasswordError::UnexpectedError(e.into()))
    .map_err(|_| ChangePasswordError::IncorrectCurrentPassword)?;
    // トランザクションを開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ChangePasswordError::UnexpectedError(e.into()))?;
    // パスワードを変更
    let hashed_password =
        HashedPassword::new(&new_password).map_err(ChangePasswordError::UnexpectedError)?;
    PgUserRepository::default()
        .change_password(user.id(), hashed_password, &mut tx)
        .await
        .map_err(|e| match e {
            UserRepositoryError::UnexpectedError(e) => ChangePasswordError::UnexpectedError(e),
            UserRepositoryError::NotFoundError(e) => ChangePasswordError::NotFound(e),
            _ => ChangePasswordError::UnexpectedError(anyhow!(
                "パスワード変更する機能に、実装上のエラーがあります。"
            )),
        })?;
    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| ChangePasswordError::UnexpectedError(e.into()))?;
    // Redisからセッションデータを削除
    session.purge();

    Ok(())
}
