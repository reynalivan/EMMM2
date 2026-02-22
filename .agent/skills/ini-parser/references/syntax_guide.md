# 3DMigoto Syntax Guide (Definitive)

## 1. Variable Scopes & Persistence
-   `$variable = 0`: Local variable (reset on reload).
-   `global $variable`: Shared across INI files.
-   `global persist $variable`: Saved to `d3dx_user.ini` (survives game restart).
-   **Note**: Variables can be declared in `[Constants]` OR implied in `[Key...]` sections (common in older mods).

## 2. Resource Definitions
Resources map hashes to files.
```ini
[ResourceModelName]
type = Buffer
stride = 40
filename = ModelName.buf

[ResourceTextureName]
filename = TextureName.dds
```
-   **Buffers**: Have `stride` and `type = Buffer`.
-   **Textures**: Just `filename`.

## 3. TextureOverrides & Logic
A logical block for a specific hash.
-   **Duplicates**: You can have multiple sections with the SAME name. 3DMigoto merges them or executes them sequentially.
-   **Match First**: `match_first_index = 0` is used to target specific sub-meshes within a hash.

## 4. Merged Mod Logic (CommandList)
Merged mods use `CommandList` to act as a "Router".
```ini
[TextureOverrideMerged]
hash = 1234abcd
run = CommandListSelector

[CommandListSelector]
if $swapvar == 0
    vb0 = ResourceVariantA
else if $swapvar == 1
    vb0 = ResourceVariantB
endif
```
-   **Scale**: Can have 50+ `else if` branches.
-   **Nesting**: `if` inside `if` is allowed (seen in `RaidenShogun.ini` for `dress` vs `nipple` logic).

## 5. Advanced Key Mapping
Mods use `[Key...]` sections to define input handlers.

### Interaction Types
1.  **Cycle**: Steps through values.
    ```ini
    type = cycle
    $var = 0,1,2,3,4,5,6,7,8,9,10  ; Can be long!
    ```
2.  **Toggle**: Switches between current and last value (or 0/1).
3.  **Hold**: Active only while key is pressed.

### Key Properties
-   **condition**: `condition = $active == 1` (Standard guard).
-   **key**: The primary activation key (e.g., `y`, `ctrl up`, `shift right`).
-   **back**: (Optional) Key to cycle backwards (e.g., `back = [`).
-   **type**: `cycle`, `toggle`, `hold`.

## 6. Critical Parsing Rules ("Lossless")
1.  **Line-Based Only**: Never parse into a tree and write back. The grammar is too loose and permits duplicates/comments that standard libraries destroy.
2.  **Order Matters**: `run = CommandListA` followed by `run = CommandListB` executes in order.
3.  **Case Insensitive**: `ELSE IF` == `else if`.
4.  **Whitespace**: ` $var = 1` is valid.
