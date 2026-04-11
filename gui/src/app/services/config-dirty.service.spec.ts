import { ConfigDirtyService } from './config-dirty.service';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';
import { TestBed } from '@angular/core/testing';

describe('ConfigDirtyService', () => {
  let service: ConfigDirtyService;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock({
      'has_unsaved_config_changes': false,
    });
    TestBed.configureTestingModule({});
    service = TestBed.inject(ConfigDirtyService);
  });

  it('should start as not dirty', () => {
    expect(service.isDirty()).toBe(false);
  });

  describe('markDirty', () => {
    it('should set isDirty to true', () => {
      // When
      service.markDirty();

      // Then
      expect(service.isDirty()).toBe(true);
    });
  });

  describe('markClean', () => {
    it('should set isDirty to false', () => {
      // Given
      service.markDirty();

      // When
      service.markClean();

      // Then
      expect(service.isDirty()).toBe(false);
    });
  });

  describe('syncFromBackend', () => {
    it('should set isDirty from backend when true', async () => {
      // Given
      tauri.setResponse('has_unsaved_config_changes', true);

      // When
      await service.syncFromBackend();

      // Then
      tauri.expectInvoked('has_unsaved_config_changes');
      expect(service.isDirty()).toBe(true);
    });

    it('should set isDirty from backend when false', async () => {
      // Given
      service.markDirty();
      tauri.setResponse('has_unsaved_config_changes', false);

      // When
      await service.syncFromBackend();

      // Then
      expect(service.isDirty()).toBe(false);
    });
  });
});
