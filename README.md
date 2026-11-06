# Heroes vs Monsters — Bevy recreation prototype
`cargo run --release` from this folder (Bevy 0.20, ron 0.12). It runs on Windows, Linux and macOS.
On Linux, bevy_audio needs the ALSA development headers (libasound2-dev) and pkg-config.
Progress is stored in `save.ron` in the working directory; delete it to start a new game.
Tests: `cargo test --release` runs 12 tests: the combat math, the data integrity checks, the referenced-asset check, equipment stats and the save round-trip.

## Screens (Bevy `AppState`)
| State | Original scene | What it shows |
|---|---|---|
| Menu | SceneMenu | bg_mainmenu, title, signpost. PLAY → Inn, OPTIONS toggles sound, HOW TO shows help_1..4. The 4th sign (Leaderboards on iOS) is QUIT. |
| Tavern | SceneGameMenu (the "Inn") | Innkeepers → Hire, gold chest → Store, armor table → Equip, book/quill → Skills, mugs → party, map sign → World map, MENU → main menu. Hovering a prop shows its original bar_* sign. |
| Hire | SceneSelectHero | Hero list on pedestals (scroll with the wheel, drag or ←/→). HIRE buys the selected hero for its heroes.ron price. DISMISS removes one. The party strip and the roster set the party (up to 3). |
| WorldMap | SceneMoveMap | Nodes from WorldMoveMap.xml. Click an unlocked node to walk there, click it again to fight. BOSS / ARENA / SIEGE are top-right; SUMMON / INN / STORE / SKILLS / TEAM / STASH are at the bottom. |
| Store | SceneShop | Drag store→stash to buy, stash→store to sell (half price). Click an item, then BUY. Stock comes from ShopRefresh and restocks after each win. |
| Equip | SceneUserBag | Drag stash↔slots to equip. Class and slot rules come from heroes.ron and equipment.ron, and stats update live. Click a portrait to switch hero. |
| Skills | SceneSkillList | Click a skill to see its description, click it again to learn. Hero level = points. Prerequisites, needHadPoints and exclusion flags apply. RESET refunds every point. |
| Battle | SceneMainGame | Free 2D battlefield on map_NN.jpeg with waves from waves.ron. |
| Result | — | STAGECLEAR / GAMEOVER, rewards and level-ups. OK returns to the world map. |

## Battle controls
- Left click a hero, or press 1/2/3, to select it. Shift-click adds to the selection. Drag on empty ground to box-select.
- With heroes selected, left- or right-click the ground to move there. Click an enemy to force it as the target.
- Q / W / E, or the skill buttons at the bottom, cast each hero's learned active skill. The dark overlay shows the cooldown.
- A toggles auto-battle (on by default: heroes engage anything, and skills auto-cast).
- P or the pause button (top-right) pauses. R restarts the battle. Esc retreats to the world map.

## Economy
- Kill reward: exp = Exp + F_Exp(L−1); money = Money + F_Money(min(L−1, 40)). Money is stored ×4.
- Every party hero gets the battle's exp on a win. XP to the next level is F_Exp(level), and each level gives 1 skill point.
- A win on the furthest node unlocks the next one.

## Modules
| Module | Contents |
|---|---|
| main.rs | States, window (1136×640 with the hvsMiphon5 frame pillars), music per state, tests |
| ui.rs | Sprite UI kit, hotspots, toasts |
| common.rs | Portrait strip, description box, drag & drop item grids |
| anim.rs | Pre-rendered unit sheets |
| audio.rs | Music and SFX |
| menu, inn, hire, worldmap, store, equip, skills, battle, result | One file per screen |
| stats.rs | Equipment and passive stats |
| combat.rs | Formulas |
| data.rs, save.rs, types.rs | Data loading, save file, data schema |

## Assets
Everything under assets/ was decoded from the APK:
- ui/: UI_2x textures
- maps/
- audio/
- fonts/hvsm.ttf
- icons/: item and skill icons
- sprites/: unit sheets rendered from the .acb skeletal animations by /workspace/hvsm/anim/batch.py

## Update (Oct 31 2026)
- Renderer: Windows now defaults to Vulkan (implicit Vulkan layers disabled for the process). DX12 drew Bevy 0.20 sprites
  with each other's transforms on the GTX 1660 Ti laptop. `WGPU_BACKEND` still overrides.
- Clicks are resolved in PreUpdate (`click_system` + `generic_actions`), so screens no longer swallow BACK/PLAY clicks.
- English: UI art from the English 3.4.3 APK (`assets/UI_2x`; title.png → title_android.png, button_mainmenu.png →
  button_mainmenu1.png), font Gill Sans Ultra Bold (`fonts/gilsanub.ttf`, the English build's font), names and
  descriptions from its plain-XML `assets/gamedata` (see `/workspace/hvsm/english.py`, `english_sets.py`). The RON
  files keep the Chinese originals in `name_zh` / `introduction_zh` / `intro_zh` / `class_name_zh`.
- Waves: kills are counted as spawned minus alive (corpses despawn after 2.5 s, which stalled wave 2).
- Self-test: `HVSM_AUTOTEST="w5;smenu;c0.768,0.275;w2;sinn;q"` (w=wait s, c=click at window fraction, k=key, s=screenshot
  to `$HVSM_SHOTS`, q=quit). The laptop helper is `C:\Users\benja\a.ps1 -script "..."`.

- Enemy names (`name` in enemies.ron) come from the designers' comments in the English 3.4.3 `EnemyAttribute.xml`, which has no name field. They are generated by `/workspace/hvsm/enemy_names.py` and shown above each enemy's HP bar.
- Inn signs (STORE, SKILLS, ...) show only while a prop is pressed, as on the original touch build, not on mouse hover.
