import { ConfigDirtyService } from './config-dirty.service';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';
import { TestBed } from '@angular/core/testing';

describe('ConfigDirtyService', () => {
  let service: ConfigDirtyService;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock({
      'get_config_diff': { modified_tasks: [], modified_variables: [] },
    });
    TestBed.configureTestingModule({});
    service = TestBed.inject(ConfigDirtyService);
  });

  it('should start clean', () => {
    expect(service.isDirty()).toBe(false);
    expect(service.modifiedTasks().size).toBe(0);
    expect(service.modifiedVariables().size).toBe(0);
  });

  describe('markTaskDirty', () => {
    it('should add task name to modified set and make dirty', () => {
      service.markTaskDirty('task_a');

      expect(service.isDirty()).toBe(true);
      expect(service.isTaskModified('task_a')).toBe(true);
      expect(service.isTaskModified('task_b')).toBe(false);
    });

    it('should accumulate multiple task names', () => {
      service.markTaskDirty('task_a');
      service.markTaskDirty('task_b');

      expect(service.modifiedTasks().size).toBe(2);
    });
  });

  describe('markVariableDirty', () => {
    it('should add variable name to modified set and make dirty', () => {
      service.markVariableDirty('var_a');

      expect(service.isDirty()).toBe(true);
      expect(service.isVariableModified('var_a')).toBe(true);
      expect(service.isVariableModified('var_b')).toBe(false);
    });
  });

  describe('markClean', () => {
    it('should clear both sets', () => {
      service.markTaskDirty('task_a');
      service.markVariableDirty('var_a');

      service.markClean();

      expect(service.isDirty()).toBe(false);
      expect(service.modifiedTasks().size).toBe(0);
      expect(service.modifiedVariables().size).toBe(0);
    });
  });

  describe('syncFromBackend', () => {
    it('should populate both sets from diff', async () => {
      tauri.setResponse('get_config_diff', {
        modified_tasks: ['task_a', 'task_b'],
        modified_variables: ['var_a'],
      });

      await service.syncFromBackend();

      tauri.expectInvoked('get_config_diff');
      expect(service.isDirty()).toBe(true);
      expect(service.isTaskModified('task_a')).toBe(true);
      expect(service.isTaskModified('task_b')).toBe(true);
      expect(service.isVariableModified('var_a')).toBe(true);
    });

    it('should clear sets when backend reports no changes', async () => {
      service.markTaskDirty('task_a');
      tauri.setResponse('get_config_diff', {
        modified_tasks: [],
        modified_variables: [],
      });

      await service.syncFromBackend();

      expect(service.isDirty()).toBe(false);
    });
  });
});
