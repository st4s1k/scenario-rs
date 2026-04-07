import { TestBed } from '@angular/core/testing';
import { FormControl } from '@angular/forms';
import { AppComponent, Step, Task, OnFailStep } from './app.component';
import { ExecutionStateService } from './services/execution-state.service';
import { setupTauriMock, TauriTestHarness } from './testing/tauri-mocks';

function makeRequiredField(overrides: Partial<any> = {}): any {
  return {
    label: 'App version',
    var_type: 'text',
    value: '1.0.0',
    read_only: false,
    ...overrides,
  };
}

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    description: 'Deploy',
    error_message: 'Deploy failed',
    task_type: 'RemoteSudo',
    command: 'echo hi',
    ...overrides,
  };
}

function makeStep(overrides: Partial<Step> = {}): Step {
  return {
    index: 0,
    task: makeTask(),
    on_fail_steps: [],
    ...overrides,
  };
}

describe('AppComponent', () => {
  let component: AppComponent;
  let tauri: TauriTestHarness;
  let executionStateService: jasmine.SpyObj<ExecutionStateService>;

  beforeEach(() => {
    tauri = setupTauriMock({
      'get_config_path': '/path/to/config.toml',
      'is_valid_config_path': true,
      'load_config': undefined,
      'get_required_variables': {},
      'get_resolved_variables': {},
      'get_tasks': {},
      'get_steps': [],
      'execute_scenario': undefined,
      'update_required_variables': undefined,
    });

    executionStateService = jasmine.createSpyObj('ExecutionStateService', ['init', 'destroy', 'isExecuting'], {
      isExecuting: jasmine.createSpy('isExecuting').and.returnValue(false),
      executionState: jasmine.createSpy('executionState').and.returnValue(null),
    });

    TestBed.configureTestingModule({
      providers: [
        { provide: ExecutionStateService, useValue: executionStateService },
      ],
    });

    component = TestBed.runInInjectionContext(() => new AppComponent());
  });

  afterEach(() => {
    component?.ngOnDestroy();
  });

  describe('fetchConfigPath', () => {
    it('should invoke get_config_path and set form value', async () => {
      // Given
      tauri.setResponse('get_config_path', '/my/config.toml');

      // When
      await component.fetchConfigPath();

      // Then
      tauri.expectInvoked('get_config_path');
      expect(component.scenarioConfigPath.value).toBe('/my/config.toml');
    });
  });

  describe('clearLog', () => {
    it('should reset execution log to empty', () => {
      // Given
      component.executionLog.set('Some log content');

      // When
      component.clearLog();

      // Then
      expect(component.executionLog()).toBe('');
    });
  });

  describe('executeScenario', () => {
    it('should invoke execute_scenario command', () => {
      // Given & When
      component.executeScenario();

      // Then
      tauri.expectInvoked('execute_scenario');
    });
  });

  describe('loadConfig', () => {
    it('should invoke load_config with the current path', async () => {
      // Given
      component.scenarioConfigPath.setValue('/test/scenario.toml');

      // When
      await component.loadConfig();

      // Then
      tauri.expectInvoked('load_config', { configPath: '/test/scenario.toml' });
    });

    it('should fetch tasks, steps, variables after loading', async () => {
      // Given
      component.scenarioConfigPath.setValue('/test/scenario.toml');

      // When
      await component.loadConfig();

      // Then
      tauri.expectInvoked('get_tasks');
      tauri.expectInvoked('get_steps');
      tauri.expectInvoked('get_required_variables');
      tauri.expectInvoked('get_resolved_variables');
    });
  });

  describe('selectConfigFile', () => {
    it('should not load config when dialog is cancelled', async () => {
      // Given
      tauri.setResponse('plugin:dialog|open', null);

      // When
      await component.selectConfigFile();

      // Then
      tauri.expectInvoked('plugin:dialog|open');
      tauri.expectNotInvoked('load_config');
    });

    it('should set config path and load when file is selected', async () => {
      // Given
      tauri.setResponse('plugin:dialog|open', '/selected/file.toml');

      // When
      await component.selectConfigFile();

      // Then
      expect(component.scenarioConfigPath.value).toBe('/selected/file.toml');
      tauri.expectInvoked('load_config', { configPath: '/selected/file.toml' });
    });
  });

  describe('selectRequiredFile', () => {
    it('should update required field value when file selected', async () => {
      // Given
      component.requiredFields = { 'app_path': makeRequiredField({ label: 'App Path', value: '' }) };
      component.requiredFieldsFormGroup.addControl('app_path', new FormControl(''));
      tauri.setResponse('plugin:dialog|open', '/selected/app.jar');

      // When
      await component.selectRequiredFile('app_path');

      // Then
      expect(component.requiredFields['app_path'].value).toBe('/selected/app.jar');
      expect(component.requiredFieldsFormGroup.controls['app_path'].value).toBe('/selected/app.jar');
    });

    it('should not update when dialog is cancelled', async () => {
      // Given
      component.requiredFields = { 'app_path': makeRequiredField({ label: 'App Path', value: 'original' }) };
      component.requiredFieldsFormGroup.addControl('app_path', new FormControl('original'));
      tauri.setResponse('plugin:dialog|open', null);

      // When
      await component.selectRequiredFile('app_path');

      // Then
      expect(component.requiredFields['app_path'].value).toBe('original');
    });
  });

  describe('validatePathAndLoadConfig', () => {
    it('should clear errors for empty path', async () => {
      // Given
      component.scenarioConfigPath.setValue('');

      // When
      await component.validatePathAndLoadConfig();

      // Then
      expect(component.scenarioConfigPath.errors).toBeNull();
    });

    it('should clear errors for whitespace-only path', async () => {
      // Given
      component.scenarioConfigPath.setValue('   ');

      // When
      await component.validatePathAndLoadConfig();

      // Then
      expect(component.scenarioConfigPath.errors).toBeNull();
    });

    it('should load config for valid path', async () => {
      // Given
      component.scenarioConfigPath.setValue('/valid/path.toml');
      tauri.setResponse('is_valid_config_path', true);

      // When
      await component.validatePathAndLoadConfig();

      // Then
      tauri.expectInvoked('is_valid_config_path', { path: '/valid/path.toml' });
    });

    it('should set invalidPath error for invalid path', async () => {
      // Given
      component.scenarioConfigPath.setValue('/invalid/path.toml');
      tauri.setResponse('is_valid_config_path', false);

      // When
      await component.validatePathAndLoadConfig();

      // Then
      expect(component.scenarioConfigPath.errors).toEqual({ invalidPath: true });
    });
  });

  describe('updateRequiredVariables', () => {
    it('should invoke with collected variable values', async () => {
      // Given
      component.requiredFields = {
        'version': makeRequiredField({ value: '2.0' }),
        'env': makeRequiredField({ value: 'prod' }),
      };

      // When
      await component.updateRequiredVariables();

      // Then
      tauri.expectInvoked('update_required_variables', {
        requiredVariables: { version: '2.0', env: 'prod' },
      });
    });
  });

  describe('flushBufferedLog', () => {
    it('should join buffered messages with newlines', () => {
      // Given
      (component as any).pendingLogBuffer = ['line1', 'line2', 'line3'];
      component.executionLog.set('');

      // When
      (component as any).flushBufferedLog();

      // Then
      expect(component.executionLog()).toBe('line1\nline2\nline3');
    });

    it('should append to existing log with newline separator', () => {
      // Given
      component.executionLog.set('existing');
      (component as any).pendingLogBuffer = ['new'];

      // When
      (component as any).flushBufferedLog();

      // Then
      expect(component.executionLog()).toBe('existing\nnew');
    });

    it('should truncate to 1MB when log exceeds max size', () => {
      // Given
      const largeLog = 'x'.repeat(900_000);
      component.executionLog.set(largeLog);
      const overflow = 'y'.repeat(200_000);
      (component as any).pendingLogBuffer = [overflow];

      // When
      (component as any).flushBufferedLog();

      // Then
      expect(component.executionLog().length).toBeLessThanOrEqual(1_000_000);
    });

    it('should clear the buffer after flushing', () => {
      // Given
      (component as any).pendingLogBuffer = ['msg'];

      // When
      (component as any).flushBufferedLog();

      // Then
      expect((component as any).pendingLogBuffer.length).toBe(0);
    });
  });

  describe('getResolvedVariables', () => {
    it('should populate resolvedVariables from invoke', async () => {
      // Given
      tauri.setResponse('get_resolved_variables', { host: 'prod-server', port: '8080' });

      // When
      await (component as any).getResolvedVariables();

      // Then
      expect(component.resolvedVariables).toEqual({ host: 'prod-server', port: '8080' });
    });

    it('should default to empty object when invoke returns null', async () => {
      // Given
      tauri.setResponse('get_resolved_variables', null);

      // When
      await (component as any).getResolvedVariables();

      // Then
      expect(component.resolvedVariables).toEqual({});
    });
  });

  describe('getTasks', () => {
    it('should populate tasks from invoke', async () => {
      // Given
      const tasks = { deploy: makeTask() };
      tauri.setResponse('get_tasks', tasks);

      // When
      await (component as any).getTasks();

      // Then
      expect(component.tasks).toEqual(tasks);
    });

    it('should default to empty object when invoke returns null', async () => {
      // Given
      tauri.setResponse('get_tasks', null);

      // When
      await (component as any).getTasks();

      // Then
      expect(component.tasks).toEqual({});
    });
  });

  describe('getSteps', () => {
    it('should populate steps from invoke', async () => {
      // Given
      const steps = [makeStep()];
      tauri.setResponse('get_steps', steps);

      // When
      await (component as any).getSteps();

      // Then
      expect(component.steps).toEqual(steps);
    });

    it('should default to empty array when invoke returns null', async () => {
      // Given
      tauri.setResponse('get_steps', null);

      // When
      await (component as any).getSteps();

      // Then
      expect(component.steps).toEqual([]);
    });
  });

  describe('isInvalidScenarioConfigPath', () => {
    it('should return false for pristine control', () => {
      // Given & When & Then
      expect(component.isInvalidScenarioConfigPath).toBe(false);
    });
  });

  describe('configPathValidator', () => {
    it('should return null for empty value', (done) => {
      // Given
      const validator = component.configPathValidator();
      const control = { value: '' } as any;

      // When
      const result$ = validator(control);
      (result$ as any).subscribe((result: any) => {
        // Then
        expect(result).toBeNull();
        done();
      });
    });
  });

  describe('ngOnInit', () => {
    it('should fetch config path and initialize services', async () => {
      // Given
      tauri.setResponse('get_config_path', '/init/path.toml');

      // When
      component.ngOnInit();
      await new Promise(resolve => setTimeout(resolve, 50));

      // Then
      tauri.expectInvoked('get_config_path');
      expect(executionStateService.init).toHaveBeenCalled();
    });
  });

  describe('setupLogUpdatesListener', () => {
    it('should register log-message event listener', async () => {
      // Given & When
      await (component as any).setupLogUpdatesListener();

      // Then
      tauri.expectInvoked('plugin:event|listen');
      const calls = tauri.invokeSpy.calls.allArgs();
      const listenCall = calls.find((a: any[]) => a[0] === 'plugin:event|listen');
      expect(listenCall![1].event).toBe('log-message');
    });

    it('should buffer log messages and flush after timeout', async () => {
      // Given
      await (component as any).setupLogUpdatesListener();

      // When
      tauri.emitEvent('log-message', 'first log line');
      tauri.emitEvent('log-message', 'second log line');

      await new Promise(resolve => setTimeout(resolve, 150));

      // Then
      expect(component.executionLog()).toContain('first log line');
      expect(component.executionLog()).toContain('second log line');
    });
  });

  describe('setupFormValueChangeListener', () => {
    it('should subscribe to form value changes', async () => {
      // Given
      component.requiredFields = { 'version': makeRequiredField({ value: '1.0' }) };
      component.requiredFieldsFormGroup.addControl('version', new FormControl('1.0'));

      // When
      await (component as any).setupFormValueChangeListener();
      component.requiredFieldsFormGroup.controls['version'].setValue('2.0');

      await new Promise(resolve => setTimeout(resolve, 350));

      // Then
      expect(component.requiredFields['version'].value).toBe('2.0');
      tauri.expectInvoked('update_required_variables');
    });
  });

  describe('ngOnDestroy', () => {
    it('should cleanup subscriptions and call service destroy', async () => {
      // Given
      await (component as any).setupLogUpdatesListener();
      await (component as any).setupFormValueChangeListener();

      // When
      component.ngOnDestroy();

      // Then
      expect(executionStateService.destroy).toHaveBeenCalled();
    });
  });

  describe('getRequiredVariables', () => {
    it('should create form controls for non-read-only fields', async () => {
      // Given
      tauri.setResponse('get_required_variables', {
        'editable': makeRequiredField({ label: 'Editable', value: 'val1', read_only: false }),
        'readonly': makeRequiredField({ label: 'ReadOnly', value: 'val2', read_only: true }),
      });

      // When
      await (component as any).getRequiredVariables();

      // Then
      expect(component.requiredFieldsFormGroup.controls['editable']).toBeDefined();
      expect(component.requiredFieldsFormGroup.controls['readonly']).toBeUndefined();
    });
  });

  describe('stepExecStates', () => {
    it('should return empty array when executionState is null', () => {
      // Given & When
      const result = component.stepExecStates();

      // Then
      expect(result).toEqual([]);
    });
  });

  describe('isInvalidScenarioConfigPath', () => {
    it('should return true when invalid and dirty', () => {
      // Given
      component.scenarioConfigPath.setValue('');
      component.scenarioConfigPath.markAsDirty();
      component.scenarioConfigPath.setErrors({ invalidPath: true });
      Object.defineProperty(component.scenarioConfigPath, 'pending', { value: false, configurable: true });

      // When
      const result = component.isInvalidScenarioConfigPath;

      // Then
      expect(result).toBe(true);
    });

    it('should return true when invalid and touched', () => {
      // Given
      component.scenarioConfigPath.setValue('');
      component.scenarioConfigPath.markAsTouched();
      component.scenarioConfigPath.setErrors({ invalidPath: true });
      Object.defineProperty(component.scenarioConfigPath, 'pending', { value: false, configurable: true });

      // When
      const result = component.isInvalidScenarioConfigPath;

      // Then
      expect(result).toBe(true);
    });
  });

  describe('selectRequiredFile', () => {
    it('should use <unknown> fallback when label is empty', async () => {
      // Given
      component.requiredFields['test_field'] = makeRequiredField({ label: '' });
      component.requiredFieldsFormGroup.addControl('test_field', new FormControl(''));
      tauri.setResponse('plugin:dialog|open', '/selected/path');

      // When
      await component.selectRequiredFile('test_field');

      // Then
      tauri.expectInvoked('plugin:dialog|open');
    });
  });

  describe('configPathValidator', () => {
    it('should return invalidPath error when path is invalid', (done) => {
      // Given
      tauri.setResponse('is_valid_config_path', false);
      const validator = component.configPathValidator();
      const control = new FormControl('invalid/path.toml');

      // When
      const result$ = validator(control);

      // Then
      (result$ as any).subscribe((result: any) => {
        expect(result).toEqual({ invalidPath: true });
        done();
      });
    });
  });
});
