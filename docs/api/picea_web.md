---
title: picea-web API 文档
---

# picea-web WebAssembly 接口

`picea-web` 将 `picea` 物理引擎封装为 WebAssembly 模块，提供面向 TypeScript/JavaScript 的高层 API。本节列出所有公开导出、数据结构及使用示例，便于在浏览器或 Node.js 环境中构建 2D 物理应用。

## 1. 基本概念

- **值类型包装**：
  - `Tuple2`：`{ x: number, y: number }`，用于点与向量。
  - `WebVector`、`WebPoint`：对应 TypeScript 类型 `Vector`、`Point`。
  - `WebMeta` / `OptionalWebMeta`：元素物理属性；对应 `Meta` 与 `MetaPartial`。
  - `WebJoinConstraintConfig` / `OptionalWebJoinConstraintConfig`：关节配置。
- **Scene 句柄**：`WebScene` 持有内部 `Scene` 指针，并通过 `wasm_bindgen` 自动处理可变访问（内部使用 `UnsafeCell`）。
- **约束句柄**：`PointConstraint` 与 `JoinConstraint` 对象提供引用操作与销毁函数。

## 2. 初始化流程

```typescript
import init, { setPanicConsoleHook, create_scene } from "picea-web";

async function bootstrap() {
  await init();
  setPanicConsoleHook();

  const scene = create_scene();
  scene.setGravity({ x: 0, y: 9.8 });

  const ground = scene.createRect(0, 580, 800, 40, {
    mass: 1000,
    isFixed: true,
    factorFriction: 0.8,
  });

  const ball = scene.createCircle(400, 100, 20, {
    mass: 5,
    factorRestitution: 0.6,
  });

  function step() {
    scene.tick(1 / 60);
    requestAnimationFrame(step);
  }

  step();
}

bootstrap();
```

## 3. 全局导出

| 函数 | 说明 |
| --- | --- |
| `set_panic_console_hook()` | 将 Rust panic 重定位到浏览器控制台，便于调试 |
| `create_scene()` | 创建新的 `WebScene` 实例 |
| `is_point_valid_add_into_polygon(point, vertices)` | 验证新增顶点是否破坏多边形合法性 |

## 4. `WebScene` 接口

### 4.1 世界配置

| 方法 | 说明 |
| --- | --- |
| `setGravity(WebVector)` | 修改默认重力向量 |
| `enableSleepMode()` / `disableSleepMode()` | 控制睡眠优化 |
| `clear()` | 重置场景，包括元素、约束与帧计数 |
| `frameCount()` | 返回累计模拟帧数 |

### 4.2 时间推进与事件

| 方法 | 说明 |
| --- | --- |
| `tick(delta_t: number)` | 推进世界时间（内部会限制 Delta 范围） |
| `registerElementPositionUpdateCallback(callback)` | 注册位置更新回调，返回回调 ID |
| `unregisterElementPositionUpdateCallback(id)` | 移除回调 |
| `forEachElement(callback)` | 遍历所有元素的几何信息（多边形或圆），回调接收 `Shape` 对象 |

### 4.3 元素创建与管理

| 方法 | 说明 |
| --- | --- |
| `createRect(x, y, width, height, meta?)` | 新增矩形，顶点坐标以左上角为基准 |
| `createCircle(cx, cy, radius, meta?)` | 新增圆形 |
| `createRegularPolygon(x, y, edgeCount, radius, meta?)` | 新增正多边形 |
| `createPolygon(vertices, meta?)` | 新增自定义凹/凸多边形 |
| `createLine(start: WebPoint, end: WebPoint, meta?)` | 新增线段 |
| `cloneElement(elementId, metaOverride?)` | 克隆现有元素形状，并可覆盖元数据 |
| `hasElement(id)` | 是否存在指定元素 |
| `removeElement(id)` | 删除元素 |
| `elements_iter()` (通过 `forEachElement`) | 遍历元素 |

### 4.4 元素查询

| 方法 | 说明 |
| --- | --- |
| `getElementMetaData(id)` | 获取元素 `Meta`，返回 `WebMeta`（可直接解构） |
| `updateElementMeta(id, metaPartial)` | 通过部分配置更新元数据（透明、固定、质量、速度等） |
| `getElementVertices(id)` | 返回元素所有顶点（线段返回起点列表，圆形暂未实现） |
| `getElementCenterPoint(id)` | 返回元素中心点 |
| `getElementRawRustCode(id)` | 生成当前元素的 Rust 构造代码快照 |
| `isPointInsideElement(x, y, id)` | 判断点是否位于元素内部 |
| `getElementKinetic(id)` | 返回能量估计（`[v², ω², Δx², Δθ]`） |
| `getSleepingStatus(id)` | 返回元素是否处于睡眠状态 |
| `element_ids()` | 获取所有元素 ID |
| `getPositionFixMap()` | 返回碰撞位移修正调试数据 |
| `isElementCollide(aId, bId, query_from_manifold?)` | 查询碰撞状态，可选择是否使用当前接触流形缓存 |

### 4.5 约束管理

| 方法 | 说明 |
| --- | --- |
| `createPointConstraint(elementId, elementPoint, fixedPoint, config?)` | 生成点-锚约束，返回 `PointConstraint` 对象 |
| `pointConstraints()` | 获取当前所有点约束句柄列表 |
| `createJoinConstraint(elementAId, pointA, elementBId, pointB, config?)` | 生成元素间关节，返回 `JoinConstraint` 对象 |
| `joinConstraints()` | 获取当前所有关节句柄 |

## 5. 约束句柄

### 5.1 `PointConstraint`

- `config()`：返回 `WebJoinConstraintConfig`，反映当前配置。
- `updateMovePoint(point)`：移动固定锚点。
- `getPointPair()`：返回元素锚点与固定点（两个 `WebPoint`）。
- `updateConfig(OptionalWebJoinConstraintConfig)`：部分更新配置。
- `dispose()`：销毁约束并自动从场景中移除。

### 5.2 `JoinConstraint`

- `config()`：返回现有配置。
- `getPointPair()`：返回绑定在元素 A/B 上的移动点。
- `updateConfig(OptionalWebJoinConstraintConfig)`：更新配置参数。
- `dispose()`：销毁约束并解除锚点。

> 两种约束均在内部记录 `is_dispose` 状态，重复调用 `dispose()` 为安全操作（无副作用）。

## 6. TypeScript 类型声明

`crates/picea-web/src/type.d.ts` 随编译结果一并导出，核心类型如下：

- `type Vector = { x: number; y: number }`
- `type Point = { x: number; y: number }`
- `type Meta` / `type MetaPartial`
- `type JoinConstraintConfig` / `type JoinConstraintConfigPartial`
- `type Shape = { id: number; centerPoint: Point; shapeType: "circle" | "polygon"; ... }`
- `interface WebScene` 补充 `forEachElement`、`registerElementPositionUpdateCallback` 等回调接口。

在 TypeScript 项目中，可直接通过 `/// <reference types="picea-web" />` 或打包工具（如 Vite、Webpack）自动拾取这些类型定义，享受静态检查。

## 7. 调试与实践建议

- **panic 钩子**：生产环境可根据需要选择是否调用 `setPanicConsoleHook`，以避免在发行模式下暴露调试信息。
- **性能**：`tick` 内部使用固定迭代次数（约束解算 10 次，位置修复 20 次），如需更稳定可在 JS 层控制帧率。
- **内存管理**：所有返回的约束句柄都持有对 `WebScene` 的引用，务必在不需要后调用 `dispose()` 释放；场景 `clear()` 会自动使旧句柄失效。
- **几何调试**：结合 `forEachElement` 的多边形/圆数据与 `getPositionFixMap()`，可以构建可视化层调试碰撞与修正。

以上接口覆盖了 `picea-web` 当前所有对外可用方法。若需扩展浏览器端渲染或交互功能，可在此基础上结合 Canvas/WebGL 或其它渲染库构建上层框架。
