import { TitlebarComponent } from './titlebar.component';
import { setupTauriMock, TauriTestHarness } from '../../testing/tauri-mocks';

describe('TitlebarComponent', () => {
  let component: TitlebarComponent;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock({});
    component = new TitlebarComponent();
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
