## 1. S5-LAB planning gate

- [x] 1.1 记录Start HEAD、浏览器失败证据、S5-LAB scope与用户执行/发布授权
- [x] 1.2 完成proposal、design与revolute-lab-workbench spec
- [x] 1.3 运行OpenSpec status/strict validate/instructions apply与规划scope检查

## 2. S5-LAB-RED acceptance-as-code

- [x] 2.1 增加artifact behavior locks：scenario、revolute kind、two finite anchors、one row、determinism/free rotation/pivot
- [x] 2.2 增加server behavior locks：catalog与live/full frame authoritative passthrough
- [x] 2.3 增加old closed debug consumer unknown-variant negative lock
- [x] 2.4 增加Web UI/i18n显式revolute contract assertions
- [x] 2.5 逐条运行targeted gates并记录有效RED，排除syntax/dependency/server harness failure

## 3. S5-LAB implementation

- [x] 3.1 增加`scene_revolute.rs` schema-v1 fixture与固定runtime behavior
- [x] 3.2 接入ScenarioId/ALL/string/catalog/dispatch并保持existing scenarios兼容
- [x] 3.3 通过generic artifact/server projection使Rust contracts转GREEN
- [x] 3.4 增加两locale显式revolute/scenario labels与Web consumer contracts
- [x] 3.5 运行全部targeted Rust/Web gates并闭合失败

## 4. Verification and browser acceptance

- [x] 4.1 运行picea-lab full、workspace/check/clippy/fmt、Web profile/build/check、OpenSpec与git hygiene gates
- [x] 4.2 完成提交前真实Chrome smoke（非formal full-SHA gate）：1440x900运行revolute_pendulum到frame>=240并核对visible/DOM/network/debug facts
- [x] 4.3 保存提交前repo外截图与console/network证据并停止本轮服务
- [x] 4.4 按新增`scene_revolute.rs`同步最小AI路由，并验证route、YAML与diff hygiene

## 5. Independent review and release

- [x] 5.1 独立subagent对spec scope、Rust/Web实现、tests与残余风险做findings-first只读review
- [x] 5.2 闭合全部High/Medium并复跑受影响门禁
- [x] 5.3 准备仅批准路径的发布候选，确认main与origin/main对齐、无env secret且git hygiene通过

## External supervisor closeout（非 checkbox）

Repo tasks完成后由supervisor执行一次提交；随后针对该40位candidate SHA从头运行formal Chrome gate。只有formal browser PASS才安全集成并push `main`，不force-push。该外部receipt只写repo外与最终回复，不为记录commit/push结果生成新的自引用提交。
