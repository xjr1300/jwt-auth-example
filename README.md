# jwt-auth-example

- JWTトークンでユーザーを認証することを試行したサンプルアプリケーション
- クッキーでトークンを送受信するため、HTTPSで運用することを前提

## 仕様

### 認証ミドルウェア

- 本使用を実現する認証ミドルウェアを実装
- 認証ミドルウェは、保護されたリソースへのアクセスを許可したとき、そのユーザーをリクエストハンドラに渡す

### ユーザークレデンシャル

- ユーザーの識別にEメールアドレスを使用
- ユーザークレデンシャルに、Eメールアドレスとパスワードを使用
- パスワードにはユーザーごとに別のソルトを付与
- ソルトを付与したパスワードを、システム固定の秘密鍵(SECRET_KEY)で暗号化して保存

### クッキー

- クッキーの有効期限はブラウザセッション
  - ブラウザを閉じたらセッションが終了
- サーバーはブラウザに以下のクッキーを保存するように指示
  - セッションID
  - アクセストークン
  - リフレッシュトークン
- セッションIDの保存指示や読み込みなどの処理は、actix-sessionに移譲
 
### セッションデータの管理

- セッションIDをキーにRedisで以下のセッションデータを管理
  - ユーザーID（UUIDバージョン4）
  - アクセストークン
  - アクセストークンの有効期限（UNIXエポック秒）
  - リフレッシュトークン
  - アクセストークンの有効期限（UNIXエポック秒）
- アクセストークンの有効期限は10分（環境変数で変更可能）
- リフレッシュトークンの有効期限は60分（環境変数で変更可能）
- アクセストークンとリフレッシュトークンには以下を含める
  - sub: ユーザーID
  - exp: それぞれの有効期限を示すUNIXエポック秒
- セッションは、Redisの機能を使用して、リフレッシュトークンの有効期限まで記録

### ブラウザによるトークンの送信

- クッキーは`HttpOnly`を設定するため、JavaScriptでクッキーにアクセスできない
- トークンのサイレントリフレッシュを自動的に実施するために、アクセストークンとリフレッシュトークン双方をクッキーで送信
 
### ユーザー認証

1. SPAアプリが、Eメールアドレスとパスワードを送信して、ユーザーの認証を試行
2. サーバーは、ユーザーが認証に成功したら・・・
  - ユーザーをデータベースから取得して、ユーザーが有効か確認
    - ユーザーが無効な場合、サーバーはSPAアプリに`401 Unauthorized`でレスポンス
  - サーバーは、セッションデータを生成して、リフレッシュトークンの有効期限でRedisに登録
  - サーバーは、ブラウザに認証に成功したことを応答するとともに、セッションID、アクセストークン及びリフレッシュトークンをクッキーに保存するように指示
    - HttpOnly: true
      - JavaScriptからクッキーにアクセスできない
    - SameSite: Lax
      - 他のサイトからシステムへのPOSTリクエストで、クッキーが送信されない
    - Secure: true
      - HTTPSのみクッキーを送信
3. サーバーは、ユーザーが認証に失敗した場合、成功したら・・・
  - サーバーは、SPAアプリに`401 Unauthorized`でレスポンス
  - SPAアプリは、認証ページを表示

### 保護されたAPIへのアクセス

- [1] クライアントが、保護されたAPIをリクエスト
  - ブラウザは、クッキーでセッションID、アクセストークンとリクエストトークンをサーバーに送信
- [2] サーバーは、ブラウザが送信したクッキーに記録されたセッションIDをキーにredisからセッションデータを取得
- [3] サーバーは、上記で取得したアクセストークンとブラウザが送信したアクセストークンを比較
- [4-1] アクセストークンが一致した場合
  - サーバーは、アクセストークンが有効期限内か確認
  - [4-1-1] アクセストークンが有効期限内の場合
    - サーバーは、ユーザーをデータベースから取得して、ユーザーが有効か確認
      - [4-1-1-1] ユーザーが有効な場合、サーバーは保護されたAPIへのリクエストを処理
      - [4-1-1-2] ユーザーが無効な場合、サーバーは`401 Unauthorized`で応答
  - [4-1-2] アクセストークンの有効期限が切れている場合、サーバーは、上記で取得したリフレッシュトークンとブラウザが送信したリフレッシュトークンを比較
    - [4-1-2-1] リフレッシュトークンが一致した場合
      - サーバーは、リフレッシュトークンが有効期限内か確認
      - [4-1-2-1-1] リフレッシュトークンが有効期限内の場合
        - `4-1-1-1`と同じ処理を実行
        - サーバーは、セッションデータを生成して、リフレッシュトークンの有効期限でRedisに登録
        - サーバーは、ユーザー認証時と同様に、セッションID、アクセストークン及びリフレッシュトークンをクッキーに保存するように指示
      - [4-1-2-1-2] リフレッシュトークンの有効期限が切れている場合
        - サーバーは、`401 Unauthorized`で応答
    - [4-1-2-2] リフレッシュトークンが異なる場合
      - サーバーは、`401 Unauthorized`で応答
- [4-2] アクセストークンが異なる場合
  - サーバーは、`401 Unauthorized`で応答

### パスワード変更

1. SPAアプリが、パスワード変更APIをリクエスト
2. サーバーは、ユーザーのパスワードを変更
3. サーバーは、セッションデータをRedisから削除
4. サーバーは、ブラウザにセッションID、アクセストークン及びリフレッシュトークンの有効期限を過去に変更するように指示
   - これらのクッキーが無効になる
5. サーバーは、SPAアプリに`200 OK`でレスポンス
   - クライアントは、ログアウト状態に移行

### ログアウト

1. SPAアプリが、ログアウトAPIをリクエスト
2. サーバーは、セッションデータをRedisから削除
3. サーバーは、ブラウザにセッションID、アクセストークン及びリフレッシュトークンの有効期限を過去に変更するように指示
4. サーバーは、SPAアプリに`200 OK`でレスポンス

## テスト

### 単体テスト

単体テスト以下の通り実行する。
統合テストには、`ignore`属性を付与しているため、統合テストは実行されない。

```bash
cargo test
```

### 統合テスト

統合テストは以下の通り実行する。

```bash
cargo test -- --ignored
```
