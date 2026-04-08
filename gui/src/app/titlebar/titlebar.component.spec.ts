import { TitlebarComponent } from './titlebar.component';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';

describe('TitlebarComponent', () => {
  let component: TitlebarComponent;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock({
      'get_debug_mode': false,
      'set_debug_mode': undefined,
    });
    component = new TitlebarComponent();
  });

  describe('ngOnInit', () => {
    it('should fetch debug mode on init', async () => {
      // Given
      tauri.setResponse('get_debug_mode', true);

      // When
      await component.ngOnInit();

      // Then
      tauri.expectInvoked('get_debug_mode');
      expect(component.debugMode()).toBe(true);
    });
  });

  describe('toggleDebugMode', () => {
    it('should toggle debug mode from false to true', () => {
      // Given
      component.debugMode.set(false);

      // When
      component.toggleDebugMode();

      // Then
      expect(component.debugMode()).toBe(true);
      tauri.expectInvoked('set_debug_mode', { debugMode: true });
    });

    it('should toggle debug mode from true to false', () => {
      // Given
      component.debugMode.set(true);

      // When
      component.toggleDebugMode();

      // Then
      expect(component.debugMode()).toBe(false);
      tauri.expectInvoked('set_debug_mode', { debugMode: false });
    });
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
