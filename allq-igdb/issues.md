Ran into some issues when trying to generate a Rust client

Spec: https://github.com/s-crypt/IGDB-OpenAPI

# Issues

## `mod` enum variant for `models::game_category_enums::GameCategoryEnums`

`mod` is a reserved keyword in Rust, so this is invalid/causes the build to fail

### Manual resolution

```diff
diff --git a/src/models/game_category_enums.rs b/src/models/game_category_enums.rs
index 7c7e31d..679a694 100644
--- a/src/models/game_category_enums.rs
+++ b/src/models/game_category_enums.rs
@@ -22,7 +22,8 @@ pub enum GameCategoryEnums {
     expansion = 2,
     bundle = 3,
     standalone_expansion = 4,
-    mod = 5,
+    #[serde(rename="mod")]
+    game_mod = 5,
     episode = 6,
     season = 7,
     remake = 8,
@@ -43,7 +44,7 @@ impl std::fmt::Display for GameCategoryEnums {
             Self::expansion => "2",
             Self::bundle => "3",
             Self::standalone_expansion => "4",
-            Self::mod => "5",
+            Self::game_mod => "5",
             Self::episode => "6",
             Self::season => "7",
             Self::remake => "8",
```

---

## `Id` integer primitive schema not properly generated

```rust
/// Modes of gameplay
#[serde(rename = "game_modes", skip_serializing_if = "Option::is_none")]
pub game_modes: Option<Vec<models::Id>>
```

### Manual Resolution

```rust
pub type Id = i32;
```

___

Wondering if there is any way to tweak the OpenAPI generator configuration to resolve these issues
Or if (God forbid) there are issues that would be worth opening upstream