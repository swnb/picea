# Collision And Constraints

Collision and constraint solving are intentionally split:

- `collision` finds potential and actual contact pairs.
- `constraints` turns contacts into solver state and applies impulses/corrections.
- `scene` orchestrates the order and owns the lifecycle of contact manifolds.

## Collision Pipeline

```mermaid
flowchart TD
    Store["ElementStore"] --> Wrap["CollisionalCollectionWrapper"]
    Wrap --> AabbCache["AabbCache::build"]
    AabbCache --> SAP["SweepAndPruneBroadphase(axis=X)"]
    SAP --> Candidate["BroadphasePair"]
    Candidate --> Prepare["prepare_accurate_collision_detection"]
    Prepare --> SubA{"sub_colliders?"}
    SubA --> Narrow["GjkEpaNarrowphase.detect"]
    Narrow --> ContactPairs["Vec<ContactPointPair>"]
    ContactPairs --> Scene["Scene::collision_detective"]
```

## Manifold Lifecycle

```mermaid
stateDiagram-v2
    [*] --> New: new contact pair
    New --> Active: inserted into manifold map
    Active --> PendingRefresh: continuing pair detected in current pass
    PendingRefresh --> WarmStarted: warm_start consumes cached impulse
    WarmStarted --> Active: refresh contact pairs and transfer matching cache
    Active --> Inactive: not seen in current collision pass
    Inactive --> Active: re-contact, replace contacts without old warm-start cache
    Inactive --> [*]: stale / removed by future cleanup
```

## Contact Key Transfer

`ContactPointKey` is a conservative identity for transferring cached impulses between rebuilt contact points.

```mermaid
flowchart LR
    OldInfo["previous ContactPointPairConstraintInfo"] --> OldKey["old contact_key_for_transfer"]
    NewPair["new ContactPointPair"] --> NewKey["contact_key_with_centers"]
    OldKey --> UniqueOld{"unique old key?"}
    NewKey --> UniqueNew{"unique new key?"}
    UniqueOld -- "yes" --> Match{"same key?"}
    UniqueNew -- "yes" --> Match
    Match -- "yes" --> Transfer["transfer total_lambda and total_friction_lambda"]
    Match -- "no" --> Zero["new contact starts from zero cache"]
    UniqueOld -- "no" --> Zero
    UniqueNew -- "no" --> Zero
```

Transfer rules:

- Only transfer cached `total_lambda` and `total_friction_lambda`.
- Do not transfer `real_total_lambda` or `real_total_friction_lambda`; the current frame solver recomputes real applied totals.
- Non-finite, zero-normal, degenerate, duplicate, or ambiguous keys do not transfer.
- Re-contact after an inactive pass does not inherit pre-separation lambda.

## Constraint Solve Flow

```mermaid
sequenceDiagram
    participant Scene
    participant Manifold as ContactConstraintManifold
    participant Contact as ContactConstraint
    participant ElementA
    participant ElementB

    Scene->>Manifold: mark_all_inactive()
    Scene->>Manifold: insert / queue current contact pairs
    Scene->>Contact: warm_start()
    Contact->>ElementA: apply cached impulse
    Contact->>ElementB: apply cached impulse
    Scene->>Contact: refresh_contact_point_pairs_after_warm_start()
    Scene->>Contact: pre_solve(delta_time)
    loop velocity iterations
        Scene->>Contact: solve_velocity_constraint()
        Scene->>Contact: solve_friction_constraint()
    end
    loop position fix iterations
        Scene->>Contact: solve_position_constraint()
    end
```

## Boundaries

| Area | Belongs In | Should Not Leak Into |
| --- | --- | --- |
| AABB filtering and broadphase strategy | `collision` | `constraints` solver math |
| Contact pair identity and geometry-derived keys | `collision` + `constraints/contact` | wasm API |
| Contact lifecycle timing | `scene` | shape geometry |
| Effective mass and lambda clamping | `constraints` | broadphase |
| Shape sub-collider decomposition | `shape` | solver |

## Known Design Tradeoffs

- AABB cache is rebuilt per broadphase pass; it is not yet a persistent shape cache.
- Contact identity is conservative and may drop warm-start cache rather than risk wrong transfer.
- `Scene` still uses internal raw-pointer patterns around constraint iteration; storage mutations clear manifolds to avoid stale contact pointers.
- `ContactPointPair` currently uses `Vec` contact collections; future work may reduce allocations and add stable per-feature IDs.

