const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

// Add the mock
if (!code.includes("vi.mock('@tauri-apps/plugin-dialog'")) {
    code = code.replace(
        "import ModInboxPage from './ModInboxPage';",
        "import { open as openDialog } from '@tauri-apps/plugin-dialog';\nimport ModInboxPage from './ModInboxPage';\n\nvi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));"
    );
}

// Update the test
const oldTest = `
    fireEvent.click(screen.getByRole('button', { name: 'Choose Location' }));
    expect(mockSetSettingsTab).toHaveBeenCalledWith('games');
    expect(mockSetWorkspaceView).toHaveBeenCalledWith('settings');
`;

const newTest = `
    vi.mocked(openDialog).mockResolvedValueOnce('D:/New/Inbox/Path');
    vi.mocked(modInboxCommands.getSettings).mockResolvedValueOnce({
      app_language: 'en',
      app_theme: 'dark',
      discord_rpc: true,
      games: [{
        id: 'game-1',
        name: 'Game 1',
        game_type: 'GIMI',
        mod_path: 'mods',
        ready_to_move_path: null,
        game_exe: 'game.exe'
      }]
    });
    vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce();

    fireEvent.click(screen.getByRole('button', { name: 'Choose Location' }));
    
    await waitFor(() => expect(openDialog).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: 'actions.choose_location'
    }));
    await waitFor(() => expect(modInboxCommands.saveSettings).toHaveBeenCalledWith(expect.objectContaining({
      games: expect.arrayContaining([expect.objectContaining({ ready_to_move_path: 'D:/New/Inbox/Path' })])
    })));
`;

code = code.replace(oldTest.trim(), newTest.trim());

// Also wrap ModInboxPage updates in act as complained by vitest earlier (optional but good).
// Wait, the test already failed on Choose Location. Let's just fix the test.

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
