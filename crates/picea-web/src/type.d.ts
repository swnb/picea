type Vector = { x: number; y: number }
type Point = { x: number; y: number }
type Meta = {
  mass: number
  isFixed: boolean
  isTransparent: boolean
  angle: number
  velocity: { x: number; y: number }
  factorRestitution: number
  factorFriction: number
}
type MetaPartial = Partial<Meta>

type JoinConstraintConfig = {
  distance: number
  dampingRatio: number
  frequency: number
  hard: boolean
}
type JoinConstraintConfigPartial = Partial<JoinConstraintConfig>

type Shape = {
  readonly id: number
  readonly centerPoint: Point
} & (
  | {
      readonly shapeType: "circle"
      radius: number
    }
  | {
      readonly shapeType: "polygon"
      vertices: () => Point[]
    }
)

interface WebScene {
  forEachElement: (callback: (shape: Shape) => void) => void
  registerElementPositionUpdateCallback(
    callback: (
      id: number,
      translate: { x: number; y: number },
      rotation: number
    ) => void
  ): number
}
