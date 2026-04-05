---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/run.rs
  - src/wait.rs
  - tests/integration.rs
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec/spec.md
---

# 変更提案: refine-run-wait-deadline

**Change Type**: implementation

## Premise / Context
- このリポジトリは Rust 製 CLI `agent-exec` で、CLI 契約変更時は `src/main.rs`・`src/schema.rs`・`tests/integration.rs` を揃えて更新する前提がある。
- 現行 canonical spec では `run --wait` は終端状態まで無制限に待機する要件になっている。
- セッション中の要望として、`run --wait` は既定で 30 秒だけ待機し、待機上限の変更はジョブ timeout と分離した専用フラグで表したい、という方向が確認された。
- `--timeout` は既にジョブ実行時間の制限として使われているため、待機上限に流用すると意味衝突が起きる。
- 短い CLI と意味分離の両立のため、`run --wait --until <ms>` と `run --wait --forever` を導入し、既存 `wait` サブコマンドも同じ命名体系へ整合させる方針を前提にする。

## 背景 / 課題
現行の `run --wait` はジョブが終端状態になるまで無制限に待機するため、呼び出し側が「少しだけ待って、終わらなければ job_id を使って後続処理へ移る」という制御を行いにくい。また、既存の `--timeout` はジョブ実行自体の timeout を意味しており、待機上限に転用すると CLI の意味が不明瞭になる。

## 提案する変更
- `run --wait` の既定待機上限を 30,000ms に変更する。
- `run --wait --until <ms>` を追加し、待機上限を明示的に上書きできるようにする。
- `run --wait --forever` を追加し、従来どおり終端状態まで無制限に待機できるようにする。
- `wait <job_id>` サブコマンドにも `--until <ms>` と `--forever` を導入し、待機期限の命名を `run --wait` と揃える。
- `wait` の既存 `--timeout-ms` は後方互換の扱いを明示したうえで廃止導線を決めるか、同提案内で `--until` へ置換する。
- `--until` は `run` 側では `--wait` がある場合のみ有効とし、`--forever` と排他にする。
- `--timeout` は引き続きジョブ実行時間の timeout としてのみ扱い、待機上限とは分離する。
- `run --wait` と `wait` が待機上限に達した場合は、ジョブを継続実行したまま、非終端状態の `state` と `job_id` を返して終了する。

## Acceptance Criteria
- `agent-exec run --wait -- <cmd>` は最大 30,000ms 待機し、期限内に終われば終端状態と `final_snapshot` を返す。
- `agent-exec run --wait --until 1000 -- <cmd>` は最大 1,000ms 待機し、未完了なら非終端状態で返る。
- `agent-exec run --wait --forever -- <cmd>` は終端状態まで無制限に待機する。
- `agent-exec wait <job_id>` は既定で最大 30,000ms 待機するか、同一提案内で別の既定を選ぶ場合はその理由が spec に明記される。
- `agent-exec wait --until 1000 <job_id>` は最大 1,000ms 待機し、未完了なら非終端状態で返る。
- `agent-exec wait --forever <job_id>` は終端状態まで無制限に待機する。
- `agent-exec run --timeout 1000 -- <cmd>` の意味は従来どおりジョブ timeout のままで変わらない。
- `agent-exec run --until 1000 -- <cmd>` と `agent-exec run --forever -- <cmd>` は clap usage error になる。
- `agent-exec run --wait --until 1000 --forever -- <cmd>` は clap usage error になる。
- `agent-exec wait --until 1000 --forever <job_id>` は clap usage error になる。

## Out of Scope
- `run` / `wait` の JSON エンベロープ形式そのものの刷新
- ストリーミング出力や follow モードの追加
- 他サブコマンドへの待機系オプションの横展開
