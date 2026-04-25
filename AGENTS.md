# Picea AI Context Navigation

先读这两个文件，再决定动哪里：

1. `docs/ai/repo-map.md`
2. `docs/ai/index.md`

## 工作规则

- 只用 `rtk proxy` 跑 `cargo test`、`cargo check`、`cargo run`、`cargo fmt` 这类验证；没有把握时先回到 `~/.codex/RTK.md` 查约定。
- 先用当前 git/验证命令确认现场事实，再看 `docs/ai/repo-map.md` 定位模块。
- 看到 dirty/worktree 改动时，先确认来源；不要 revert、覆盖、格式化、删除不是自己这轮改出来的内容。
- 改动要贴着 milestone 走，先补行为锁或失败测试，再写最小实现。
- 只有通过对应测试门，才考虑推进下一层；不要为了赶进度偷跑到下一个 milestone。
- 如果验证结果和文档不一致，以当前仓库事实为准，并同步更新 AI 文档。

## 权威顺序

1. 当前工作区事实和验证命令输出。
2. `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` 的当前 milestone 边界和验收门。
3. `docs/ai/repo-map.md` 与 `docs/ai/doc-catalog.yaml` 的路由信息。
4. `README.md` / crate README 这类入口说明。
5. `docs/plans/2026-04-18-picea-physics-engine-milestones.md` 的归档执行历史。

## 先看哪里

- 当前生产化 milestone 目标、验收方法、执行顺序：`docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`
- 归档 milestone 历史、已完成记录：`docs/plans/2026-04-18-picea-physics-engine-milestones.md`
- 模块职责、入口、测试命令：`docs/ai/repo-map.md`
- 任务分流和问题类型路由：`docs/ai/index.md`
- 文档清单和权责：`docs/ai/doc-catalog.yaml`
- 仓库内开发规范 skills：`.agents/skills/*/SKILL.md`
