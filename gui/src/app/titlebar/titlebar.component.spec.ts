import { TitlebarComponent } from './titlebar.component';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';

describe('TitlebarComponent', () => {
  let component: TitlebarComponent;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock();
    component = new TitlebarComponent();
  });

  describe('save', () => {
    it('should invoke save_state command', () => {
      // Given & When
      component.save();

      // Then
      tauri.expectInvoked('save_state');
    });
  });

  describe('clearState', () => {
    it('should invoke clear_state command', () => {
      // Given & When
      component.clearState();

      // Then
      tauri.expectInvoked('clear_state');
    });
  });

  describe('minimize', () => {
    it('should invoke window minimize', () => {
      // Given & When
      component.minimize();

      // Then
      tauri.expectInvoked('plugin:window|minimize');
    });
  });

  describe('maximize', () => {
    it('should invoke window toggle maximize', () => {
      // Given & When
      component.maximize();

      // Then
      tauri.expectInvoked('plugin:window|toggle_maximize');
    });
  });

  describe('close', () => {
    it('should invoke window close', () => {
      // Given & When
      component.close();

      // Then
      tauri.expectInvoked('plugin:window|close');
    });
  });
});
