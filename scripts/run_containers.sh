#!/usr/bin/env bash

set -x           # シェルスクリプト内で処理した内容を表示
set -eo pipefail # パイプで複数のコマンドを繋げて実行した時、1つでもコマンドが失敗した場合は、0以外を返却

# .envファイルに記録されている環境変数を設定
source .env

# psqlコマンドの存在を確認
if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

# sqlxコマンドの存在を確認
if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 "    cargo install --version=0.6.0 sqlx-cli --no-default-features --features native-tls,postgres"
    echo >&2 "to install it."
    exit 1
fi

# 起動しているコンテナを確認
CONTAINERS=$(docker ps --filter 'name=jwt-auth-example' | sed -e '1d' | wc -l)
if [ 0 -lt $((${CONTAINERS})) ]; then
    echo >&2 "containers are already running, stop it with"
    echo >&2 "    docker-compose stop"
    exit 1
fi

# コンテナを起動
docker-compose up -d

# postgresに接続できるまで待機
until PGPASSWORD="${POSTGRES_USER_PASSWORD}" psql -h "${POSTGRES_HOST}" -U "${POSTGRES_USER_NAME}" -p "${POSTGRES_PORT}" -d "postgres" -c '\q'; do
    echo >&2 "postgres is still unavailable - sleeping"
    sleep 1
done

# データベースを作成
sqlx database create

# マイグレーションを実行
if [ -d "./migrations" ]; then
    sqlx migrate run
fi

echo >&2 "postgres has been migrated, ready to go!"
