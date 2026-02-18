# 設計メモ: agent-exec v0.1

## 1. 非同期実行とスナップショット

### 課題
`run` が `snapshot-after` で終了した後も子プロセスが継続する要件がある。`run` が終了すると stdout/stderr の読み取りが止まり、プロセスが詰まる可能性がある。

### 方針
監視プロセスを分離し、`run` は短命のフロントとして JSON を返す。監視は同一バイナリの内部サブコマンド（例: `_supervise`）で継続する。これにより、親プロセスが消えてもログと state の更新が続く。

## 2. tail/snapshot の取得方式

### 課題
リングバッファを `run` 内に保持すると、`run` 終了後に tail が生成できない。

### 方針
`stdout.log` / `stderr.log` の末尾を都度読み取りして tail を生成する。`tail-lines` と `max-bytes` の両制約で切り詰める。非 UTF-8 バイトは lossy 変換し、`encoding="utf-8-lossy"` を返す。

## 3. ディレクトリ設計と XDG

### 方針
優先順位は `--root` → `AGENT_EXEC_ROOT` → XDG data (`$XDG_DATA_HOME/agent-exec/jobs`) → 既定 (`~/.local/share/agent-exec/jobs`)。macOS でも XDG を維持する。Windows は `directories::BaseDirs::data_local_dir()` を基準にし、XDG 変数が存在する場合はそれを優先する。

## 4. Windows 対応のプロセス管理

### 方針
Windows ではプロセスグループがないため、Job Object を使用してプロセスツリーを管理する。`kill` の `--signal` は `TERM|INT|KILL` を受け付けるが、実装上は Windows の終了シグナルにマップし、未対応のシグナルは `KILL` 相当で扱う。

## 5. JSON 出力の安定性

### 方針
stdout は JSON のみを厳守し、stderr にトレーシングログを出す。エラーも JSON で `ok=false` とする。スキーマの互換性維持のため `schema_version="0.1"` を固定する。
