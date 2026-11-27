# Sound, display and game speed

Open the Grey menu and choose Settings. Grey moves focus and Blue selects.
The five rows are **Display, Sound, Speed, Save/resume and Back**.
Submenu rows show `>`; toggles display their value. Hold Grey returns one level,
restoring the parent selection. Funding is under **Session → Bankroll**;
independent count preferences are under **Training → Live counts**.
[Current illustrated menu guide](MENUS.md).

## Sound

Sound contains **Master sound, Volume, Categories, Test sounds and Back**.
Categories has Menus/Cards/Results switches. Test sounds has Deal/Win/Menu.
Master mute preserves volume and category choices and silences auditions too;
tests never turn sound on automatically.

Volume starts at 20%. In its editor, Grey lowers and Blue raises the level.
There are one-point steps from 0% through 10%, then five-point steps up to 100%.
Both ends clamp. Hold Blue auditions a deal at the selected level; hold Grey
returns to Sound. The audition does not change the training mode. Elsewhere,
hold Blue keeps its normal Off/Test/Coach/Guide mode shortcut.

Menu sounds include navigation and manual chip/max-wager controls. Card sounds
include dealing, flipping and shuffling. Results include wins, losses, pushes,
credit rollups and awards. Named tests deliberately audition their category even
if that category is disabled, while respecting master mute and zero volume.
Changing sound controls or listening to tests does not supply grading assistance.

## Display

Open **Display → Colors** to see the selected table background and card back together. Grey on StickS3 or Right on Waveshare moves between **Background**, **Card back** and **Back**. Blue or Left cycles the highlighted color and updates the preview immediately. Choose Back or hold Grey/Right to return to Display.

Backgrounds are Black, Blue, Green and Red. Card backs are Gold, Red, Blue and Charcoal. Both choices are saved. StickS3 defaults to Blue and Waveshare defaults to Black. [See every color on both devices](../README.md#display).

The **Brightness** editor uses Grey/Right to lower and Blue/Left to raise brightness in five percentage-point steps. The range is about 8–100%. **Screen** rotates the display 180°. Hold Grey/Right to go back one menu level.

## Speed

Speed has exactly two choices: **Normal** and **Fast** (the default). Select the
row to toggle. Fast shortens automatic dealing and dealer/tablemate pacing,
while retaining readable visible-card dwell and the complete card sequence.
Count questions and mistake/hand review pages remain untimed; controls and
long-press thresholds do not change. Returning from a menu restarts the current
card's readable dwell, so changing speed cannot skip a counting face.

Preferences survive a normal reboot. Optional exact resume restores the saved
preferences together with the pending session. A deliberately unsaved RAM fork
still disables persistence and resume for that boot.
