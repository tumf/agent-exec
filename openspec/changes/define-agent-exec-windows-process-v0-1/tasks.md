## 1. Windows プロセス管理

- [ ] 1.1 Job Object を用いたプロセスツリー管理を実装する（検証: Windows 実行時に子プロセスが Job Object に割り当てられる）
- [ ] 1.2 `kill` のシグナルマッピングを実装する（検証: `TERM`/`INT`/`KILL` が期待どおりに終了を誘発する）
- [ ] 1.3 `state.json` に Job Object 識別情報を記録する（検証: Windows 実行時の `state.json` に識別子が含まれる）
