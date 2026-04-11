import { TitlebarComponent } from './titlebar.component';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';

describe('TitlebarComponent', () => {
  let component: TitlebarComponent;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock({
      'get_dry_run': false,
      'set_dry_run': undefined,
    });
    component = new TitlebarComponent();
  });

  describe('ngOnInit', () => {
    it('should fetch dry run mode on init', async () => {
      // Given
      tauri.setResponse('get_dry_run', true);

      // When
      await component.ngOnInit();

      // Then
      tauri.expectInvoked('get_dry_run');
      expect(component.dryRun()).toBe(true);
    });
  });

  describe('toggleDryRun', () => {
    it('should toggle dry run from false to true', () => {
      // Given
      component.dryRun.set(false);

      // When
      component.toggleDryRun();

      // Then
      expect(component.dryRun()).toBe(true);
      tauri.expectInvoked('set_dry_run', { dryRun: true });
    });

    it('should toggle dry run from true to false', () => {
      // Given
      component.dryRun.set(true);

      // When
      component.toggleDryRun();

      // Then
      expect(component.dryRun()).toBe(false);
      tauri.expectInvoked('set_dry_run', { dryRun: false });
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
