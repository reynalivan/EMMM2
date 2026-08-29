const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.tsx', 'utf8');

// Add import for openDialog
if (!code.includes('@tauri-apps/plugin-dialog')) {
    code = code.replace(
        "import { listen } from '@tauri-apps/api/event';",
        "import { listen } from '@tauri-apps/api/event';\nimport { open as openDialog } from '@tauri-apps/plugin-dialog';"
    );
}

// Inject chooseInboxLocation
const chooseInboxLocationFn = `
  const chooseInboxLocation = async () => {
    if (!activeGameId) return;
    try {
      const selectedPath = await openDialog({
        directory: true,
        multiple: false,
        title: t('actions.choose_location'),
      });
      if (selectedPath && typeof selectedPath === 'string') {
        const settings = await modInboxCommands.getSettings();
        const game = settings.games.find((g) => g.id === activeGameId);
        if (game) {
          game.ready_to_move_path = selectedPath;
          await modInboxCommands.saveSettings(settings);
          await refresh();
        }
      }
    } catch (cause) {
      toast.error(t('errors.load', { error: formatAppError(cause) }));
    }
  };
`;

if (!code.includes('const chooseInboxLocation = async () => {')) {
    code = code.replace(
        "const openInboxSettings = () => {",
        chooseInboxLocationFn.trim() + "\n\n  const openInboxSettings = () => {"
    );
}

// Update MissingInboxState onChooseLocation prop
code = code.replace(
    "onChooseLocation={openInboxSettings}",
    "onChooseLocation={() => void chooseInboxLocation()}"
);

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.tsx', code);
