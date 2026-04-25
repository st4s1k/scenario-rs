import { TestBed } from '@angular/core/testing';
import { FormControl } from '@angular/forms';
import { AppComponent } from './app.component';
import { Task, Step } from './models/scenario.model';
import { StepExecState, TaskProgress } from './models/step-state.model';
import { ConfigDirtyService } from './services/config-dirty.service';
import { ExecutionStateService } from './services/execution-state.service';
import { StepperStep } from './warp-core/progress-stepper/progress-stepper.component';
import { TauriEventBridgeService } from './warp-core/tauri-event-bridge.service';
import { setupTauriMock, TauriTestHarness } from './testing/tauri-mocks';
import { Subject, EMPTY } from 'rxjs';

function makeRequiredField(overrides: Partial<any> = {}): any {
  return {
    label: 'App version',
    file_picker: false,
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

function makeRemoteSudoProgress(
  overrides: Partial<Extract<TaskProgress, { type: 'RemoteSudo' }>> = {}
): Extract<TaskProgress, { type: 'RemoteSudo' }> {
  return {
    type: 'RemoteSudo',
    command: 'echo hi',
    output: 'hi',
    ...overrides,
  };
}

function makeSftpCopyProgress(
  overrides: Partial<Extract<TaskProgress, { type: 'SftpCopy' }>> = {}
): Extract<TaskProgress, { type: 'SftpCopy' }> {
  return {
    type: 'SftpCopy',
    source: 'dist.zip',
    destination: '/opt/dist.zip',
    bytes_transferred: 1024,
    bytes_total: 2048,
    elapsed_ms: 2000,
    ...overrides,
  };
}

function makeStepExecState(overrides: Partial<StepExecState> = {}): StepExecState {
  return {
    index: 0,
    task_description: 'Deploy',
    status: 'Pending',
    progress: null,
    output: '',
    errors: [],
    on_fail_steps: [],
    ...overrides,
  };
}

describe('AppComponent', () => {
  let component: AppComponent;
  let tauri: TauriTestHarness;
  let executionStateService: jasmine.SpyObj<ExecutionStateService>;
  let configDirtyService: ConfigDirtyService;
  let logMessageSubject: Subject<string>;
  let logProgressSubject: Subject<string>;
  let bridgeServiceMock: any;

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
      'get_config_diff': { modified_tasks: [], modified_variables: [] },
      'get_dry_run': false,
      'set_dry_run': undefined,
      'save_state': undefined,
      'clear_state': undefined,
      'save_config': undefined,
      'discard_config_changes': undefined,
      'save_task_config': undefined,
      'discard_task_config': undefined,
      'save_variable_config': undefined,
      'discard_variable_config': undefined,
    });

    logMessageSubject = new Subject<string>();
    logProgressSubject = new Subject<string>();
    bridgeServiceMock = {
      getStream: (id: string) => {
        if (id === 'log-message') return logMessageSubject.asObservable();
        if (id === 'log-progress') return logProgressSubject.asObservable();
        return EMPTY;
      },
    };

    executionStateService = jasmine.createSpyObj('ExecutionStateService', ['init', 'destroy', 'reset', 'isExecuting'], {
      isExecuting: jasmine.createSpy('isExecuting').and.returnValue(false),
      executionState: jasmine.createSpy('executionState').and.returnValue(null),
    });

    TestBed.configureTestingModule({
      providers: [
        { provide: ExecutionStateService, useValue: executionStateService },
        { provide: TauriEventBridgeService, useValue: bridgeServiceMock },
      ],
    });

    configDirtyService = TestBed.inject(ConfigDirtyService);
    configDirtyService.markClean();
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

    it('should return cached value while validator is pending', () => {
      // Given
      component.scenarioConfigPath.setValue('');
      component.scenarioConfigPath.markAsDirty();
      component.scenarioConfigPath.setErrors({ invalidPath: true });
      Object.defineProperty(component.scenarioConfigPath, 'pending', { value: false, configurable: true });
      component.isInvalidScenarioConfigPath;
      Object.defineProperty(component.scenarioConfigPath, 'pending', { value: true, configurable: true });

      // When
      const result = component.isInvalidScenarioConfigPath;

      // Then
      expect(result).toBe(true);
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

    it('should fetch dry run on init', async () => {
      // Given
      tauri.setResponse('get_dry_run', true);

      // When
      component.ngOnInit();
      await new Promise(resolve => setTimeout(resolve, 50));

      // Then
      tauri.expectInvoked('get_dry_run');
      expect(component.dryRun()).toBe(true);
    });
  });

  describe('setupLogUpdatesListener', () => {
    it('should subscribe to log-message stream from bridge service', () => {
      // Given & When
      (component as any).setupLogUpdatesListener();

      // Then
      expect((component as any).logMessageSub).toBeTruthy();
    });

    it('should buffer log messages and flush after timeout', async () => {
      // Given
      (component as any).setupLogUpdatesListener();

      // When
      logMessageSubject.next('first log line');
      logMessageSubject.next('second log line');
      await new Promise(resolve => setTimeout(resolve, 150));

      // Then
      expect(component.executionLog()).toContain('first log line');
      expect(component.executionLog()).toContain('second log line');
    });

    it('should reset lastWasProgress on log-message', () => {
      // Given
      (component as any).setupLogUpdatesListener();
      (component as any).lastWasProgress = true;

      // When
      logMessageSubject.next('regular message');

      // Then
      expect((component as any).lastWasProgress).toBe(false);
    });
  });

  describe('setupLogProgressListener', () => {
    it('should subscribe to log-progress stream from bridge service', () => {
      // Given & When
      (component as any).setupLogProgressListener();

      // Then
      expect((component as any).logProgressSub).toBeTruthy();
    });

    it('should replace last line on consecutive progress events', () => {
      // Given
      (component as any).setupLogProgressListener();

      // When
      logProgressSubject.next('Progress: 25%');
      logProgressSubject.next('Progress: 50%');
      logProgressSubject.next('Progress: 75%');

      // Then
      expect(component.executionLog()).not.toContain('25%');
      expect(component.executionLog()).not.toContain('50%');
      expect(component.executionLog()).toContain('75%');
    });

    it('should replace single progress line without prior log', () => {
      // Given
      (component as any).setupLogProgressListener();
      logProgressSubject.next('Progress: 25%');

      // When
      logProgressSubject.next('Progress: 50%');

      // Then
      expect(component.executionLog()).toBe('Progress: 50%');
    });

    it('should append first progress line to existing log content', async () => {
      // Given
      (component as any).setupLogUpdatesListener();
      (component as any).setupLogProgressListener();
      logMessageSubject.next('Starting...');
      await new Promise(resolve => setTimeout(resolve, 150));

      // When
      logProgressSubject.next('Progress: 10%');

      // Then
      expect(component.executionLog()).toContain('Starting...');
      expect(component.executionLog()).toContain('Progress: 10%');
    });

    it('should replace progress line after log message', async () => {
      // Given
      (component as any).setupLogUpdatesListener();
      (component as any).setupLogProgressListener();
      logMessageSubject.next('Starting...');
      await new Promise(resolve => setTimeout(resolve, 150));
      logProgressSubject.next('Progress: 10%');

      // When
      logProgressSubject.next('Progress: 50%');

      // Then
      expect(component.executionLog()).toContain('Starting...');
      expect(component.executionLog()).not.toContain('10%');
      expect(component.executionLog()).toContain('Progress: 50%');
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
      (component as any).setupLogUpdatesListener();
      (component as any).setupLogProgressListener();
      await (component as any).setupFormValueChangeListener();

      // When
      component.ngOnDestroy();

      // Then
      expect(executionStateService.destroy).toHaveBeenCalled();
    });

    it('should unsubscribe log-message and log-progress subscriptions', () => {
      // Given
      (component as any).setupLogUpdatesListener();
      (component as any).setupLogProgressListener();
      const msgSpy = spyOn((component as any).logMessageSub, 'unsubscribe').and.callThrough();
      const progSpy = spyOn((component as any).logProgressSub, 'unsubscribe').and.callThrough();

      // When
      component.ngOnDestroy();

      // Then
      expect(msgSpy).toHaveBeenCalled();
      expect(progSpy).toHaveBeenCalled();
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

  describe('progressSteps', () => {
    it('should map scenario execution data into generic stepper steps', () => {
      // Given
      component.steps = [
        makeStep({
          task: makeTask({ description: 'Deploy', command: 'deploy.sh' }),
          on_fail_steps: [
            {
              index: 0,
              task: makeTask({ description: 'Rollback', command: 'rollback.sh' }),
            },
          ],
        }),
      ];
      executionStateService.executionState.and.returnValue({
        status: { kind: 'Running' },
        steps: [
          makeStepExecState({
            status: 'Running',
            errors: ['deploy failed'],
            output: 'partial output',
            progress: makeRemoteSudoProgress({ command: 'sudo deploy.sh' }),
            on_fail_steps: [
              {
                index: 0,
                task_description: 'Rollback',
                status: 'Completed',
                progress: null,
                output: 'rollback complete',
                errors: [],
              },
            ],
          }),
        ],
      });

      // When
      const result = component.progressSteps();

      // Then
      expect(result).toEqual([
        jasmine.objectContaining({
          index: 0,
          title: 'Deploy',
          status: 'Running',
          errors: ['deploy failed'],
          subSteps: [
            jasmine.objectContaining({
              index: 0,
              title: 'Rollback',
              status: 'Completed',
            }),
          ],
          data: jasmine.objectContaining({
            task: jasmine.objectContaining({ description: 'Deploy' }),
            output: 'partial output',
          }),
        }),
      ]);
    });

    it('should default to pending steps when execution state is unavailable', () => {
      // Given
      component.steps = [makeStep({ task: makeTask({ description: 'Deploy config' }) })];

      // When
      const result = component.progressSteps();

      // Then
      expect(result).toEqual([
        jasmine.objectContaining({
          title: 'Deploy config',
          status: 'Pending',
          errors: [],
        }),
      ]);
    });
  });

  describe('scenarioStepData', () => {
    it('should return typed step data when the step payload matches the scenario shape', () => {
      // Given
      const step: StepperStep = {
        index: 0,
        title: 'Deploy',
        status: 'Running',
        errors: [],
        data: {
          task: makeTask(),
          progress: null,
          output: 'done',
        },
      };

      // When
      const result = component.scenarioStepData(step);

      // Then
      expect(result).toEqual(jasmine.objectContaining({
        task: jasmine.objectContaining({ description: 'Deploy' }),
        output: 'done',
      }));
    });

    it('should return null for non-scenario step payloads', () => {
      // Given
      const step: StepperStep = {
        index: 0,
        title: 'Deploy',
        status: 'Running',
        errors: [],
        data: { unrelated: true },
      };

      // When
      const result = component.scenarioStepData(step);

      // Then
      expect(result).toBeNull();
    });
  });

  describe('formatBytes', () => {
    it('should return 0 B for zero bytes', () => {
      // Given / When / Then
      expect(component.formatBytes(0)).toBe('0 B');
    });

    it('should format non-zero byte counts using binary units', () => {
      // Given / When / Then
      expect(component.formatBytes(1536)).toBe('1.50 KB');
    });
  });

  describe('getTransferStats', () => {
    it('should return an empty string when no bytes were transferred', () => {
      // Given / When / Then
      expect(component.getTransferStats(1000, 0)).toBe('');
    });

    it('should return an empty string when the elapsed time is too small', () => {
      // Given / When / Then
      expect(component.getTransferStats(99, 1024)).toBe('');
    });

    it('should format elapsed time and throughput when enough data is available', () => {
      // Given / When / Then
      expect(component.getTransferStats(2000, 2 * 1024 * 1024)).toBe('2.0s, 1.00 MB/s');
    });
  });

  describe('transferProgress', () => {
    it('should return 0 when progress is null', () => {
      // Given / When / Then
      expect(component.transferProgress(null)).toBe(0);
    });

    it('should return 0 when progress is not an SftpCopy transfer', () => {
      // Given / When / Then
      expect(component.transferProgress(makeRemoteSudoProgress())).toBe(0);
    });

    it('should return 0 when the total byte count is zero', () => {
      // Given
      const progress = makeSftpCopyProgress({ bytes_total: 0 });

      // When / Then
      expect(component.transferProgress(progress)).toBe(0);
    });

    it('should return the rounded percent for SftpCopy transfers', () => {
      // Given
      const progress = makeSftpCopyProgress({ bytes_transferred: 1536, bytes_total: 2048 });

      // When / Then
      expect(component.transferProgress(progress)).toBe(75);
    });
  });

  describe('transferLabel', () => {
    it('should return the default label when progress is null', () => {
      // Given / When / Then
      expect(component.transferLabel(null)).toBe('0% (0 B / 0 B)');
    });

    it('should return the default label when progress is not an SftpCopy transfer', () => {
      // Given / When / Then
      expect(component.transferLabel(makeRemoteSudoProgress())).toBe('0% (0 B / 0 B)');
    });

    it('should omit transfer stats when they are unavailable', () => {
      // Given
      const progress = makeSftpCopyProgress({
        bytes_transferred: 512,
        bytes_total: 1024,
        elapsed_ms: 50,
      });

      // When / Then
      expect(component.transferLabel(progress)).toBe('50% (512.00 B / 1.00 KB)');
    });

    it('should include transfer stats when they are available', () => {
      // Given
      const progress = makeSftpCopyProgress({
        bytes_transferred: 1024 * 1024,
        bytes_total: 2 * 1024 * 1024,
        elapsed_ms: 1000,
      });

      // When / Then
      expect(component.transferLabel(progress)).toBe('50% (1.00 MB / 2.00 MB - 1.0s, 1.00 MB/s)');
    });
  });

  describe('executionError', () => {
    it('should return null when executionState is null', () => {
      // Given & When
      const result = component.executionError();

      // Then
      expect(result).toBeNull();
    });

    it('should return error when status is Failed', () => {
      // Given
      executionStateService.executionState.and.returnValue({
        status: { kind: 'Failed', error: 'connection refused' },
        steps: [],
      });

      // When
      const result = component.executionError();

      // Then
      expect(result).toBe('connection refused');
    });

    it('should return null when status is not Failed', () => {
      // Given
      executionStateService.executionState.and.returnValue({
        status: { kind: 'Completed' },
        steps: [],
      });

      // When
      const result = component.executionError();

      // Then
      expect(result).toBeNull();
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

  describe('onTaskChanged', () => {
    it('should invoke update_task, mark dirty, and refresh tasks and steps', async () => {
      // Given
      const task = makeTask({ command: 'echo updated' });
      tauri.setResponse('update_task', undefined);

      // When
      await component.onTaskChanged({ name: 'deploy', task });

      // Then
      tauri.expectInvoked('update_task', { taskName: 'deploy', task });
      tauri.expectInvoked('get_config_diff');
      tauri.expectInvoked('get_tasks');
      tauri.expectInvoked('get_steps');
    });
  });

  describe('onVariableChanged', () => {
    it('should invoke update_defined_variable, mark dirty, and refresh variables', async () => {
      // Given
      tauri.setResponse('update_defined_variable', undefined);

      // When
      await component.onVariableChanged({ name: 'host', value: 'new-server' });

      // Then
      tauri.expectInvoked('update_defined_variable', { name: 'host', value: 'new-server' });
      tauri.expectInvoked('get_config_diff');
      tauri.expectInvoked('get_resolved_variables');
    });
  });

  describe('onConfigDiscarded', () => {
    it('should refresh tasks, steps, and all variables', async () => {
      // When
      await component.onConfigDiscarded();

      // Then
      tauri.expectInvoked('get_tasks');
      tauri.expectInvoked('get_steps');
      tauri.expectInvoked('get_required_variables');
      tauri.expectInvoked('get_resolved_variables');
    });
  });

  describe('saveConfig', () => {
    it('should invoke save_config and mark clean', async () => {
      // Given
      tauri.setResponse('save_config', undefined);

      // When
      await component.saveConfig();

      // Then
      tauri.expectInvoked('save_config');
    });
  });

  // --- Titlebar domain controls (moved from TitlebarComponent) ---

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

  describe('saveState', () => {
    it('should invoke save_state command', () => {
      // Given & When
      component.saveState();

      // Then
      tauri.expectInvoked('save_state');
    });
  });

  describe('clearState', () => {
    it('should show confirm dialog', () => {
      // Given & When
      component.clearState();

      // Then
      expect(component.showClearConfirm()).toBe(true);
    });
  });

  describe('onClearConfirmed', () => {
    it('should invoke clear_state and hide dialog when confirmed', () => {
      // Given
      component.showClearConfirm.set(true);

      // When
      component.onClearConfirmed(true);

      // Then
      expect(component.showClearConfirm()).toBe(false);
      tauri.expectInvoked('clear_state');
    });

    it('should hide dialog without invoking clear_state when cancelled', () => {
      // Given
      component.showClearConfirm.set(true);

      // When
      component.onClearConfirmed(false);

      // Then
      expect(component.showClearConfirm()).toBe(false);
      tauri.expectNotInvoked('clear_state');
    });
  });

  // --- Sidebar domain actions (moved from SidebarComponent) ---

  describe('discardChanges', () => {
    it('should invoke discard_config_changes, mark clean, and refresh state', async () => {
      // Given & When
      await component.discardChanges();

      // Then
      tauri.expectInvoked('discard_config_changes');
      tauri.expectInvoked('get_tasks');
      tauri.expectInvoked('get_steps');
    });
  });

  describe('saveTask', () => {
    it('should stopPropagation, invoke save_task_config, and sync diff', async () => {
      // Given
      tauri.setResponse('get_config_diff', { modified_tasks: [], modified_variables: [] });
      const event = jasmine.createSpyObj<Event>('Event', ['stopPropagation']);

      // When
      await component.saveTask('deploy', event);

      // Then
      expect(event.stopPropagation).toHaveBeenCalled();
      tauri.expectInvoked('save_task_config', { taskName: 'deploy' });
      tauri.expectInvoked('get_config_diff');
    });
  });

  describe('discardTask', () => {
    it('should stopPropagation, invoke discard_task_config, and refresh state', async () => {
      // Given
      tauri.setResponse('get_config_diff', { modified_tasks: [], modified_variables: [] });
      const event = jasmine.createSpyObj<Event>('Event', ['stopPropagation']);

      // When
      await component.discardTask('deploy', event);

      // Then
      expect(event.stopPropagation).toHaveBeenCalled();
      tauri.expectInvoked('discard_task_config', { taskName: 'deploy' });
      tauri.expectInvoked('get_tasks');
    });
  });

  describe('saveVariable', () => {
    it('should stopPropagation, invoke save_variable_config, and sync diff', async () => {
      // Given
      tauri.setResponse('get_config_diff', { modified_tasks: [], modified_variables: [] });
      const event = jasmine.createSpyObj<Event>('Event', ['stopPropagation']);

      // When
      await component.saveVariable('host', event);

      // Then
      expect(event.stopPropagation).toHaveBeenCalled();
      tauri.expectInvoked('save_variable_config', { name: 'host' });
      tauri.expectInvoked('get_config_diff');
    });
  });

  describe('discardVariable', () => {
    it('should stopPropagation, invoke discard_variable_config, and refresh state', async () => {
      // Given
      tauri.setResponse('get_config_diff', { modified_tasks: [], modified_variables: [] });
      const event = jasmine.createSpyObj<Event>('Event', ['stopPropagation']);

      // When
      await component.discardVariable('host', event);

      // Then
      expect(event.stopPropagation).toHaveBeenCalled();
      tauri.expectInvoked('discard_variable_config', { name: 'host' });
      tauri.expectInvoked('get_resolved_variables');
    });
  });

  describe('onTaskFieldChanged', () => {
    it('should update task field and invoke update_task', async () => {
      // Given
      component.tasks = { deploy: makeTask({ command: 'echo hi' }) };
      tauri.setResponse('update_task', undefined);
      const event = { target: { value: 'echo updated' } } as unknown as Event;

      // When
      await component.onTaskFieldChanged('deploy', 'command', event);

      // Then
      tauri.expectInvoked('update_task', {
        taskName: 'deploy',
        task: jasmine.objectContaining({ command: 'echo updated' }),
      });
    });
  });

  describe('onTaskFieldInput', () => {
    it('should mark the task as dirty', () => {
      // Given
      expect(configDirtyService.isTaskModified('deploy')).toBe(false);

      // When
      component.onTaskFieldInput('deploy');

      // Then
      expect(configDirtyService.isTaskModified('deploy')).toBe(true);
    });
  });

  describe('onVariableInput', () => {
    it('should mark the variable as dirty', () => {
      // Given
      expect(configDirtyService.isVariableModified('host')).toBe(false);

      // When
      component.onVariableInput('host');

      // Then
      expect(configDirtyService.isVariableModified('host')).toBe(true);
    });
  });

  describe('modified state helpers', () => {
    it('should report whether a task is modified', () => {
      // Given
      configDirtyService.markTaskDirty('deploy');

      // When / Then
      expect(component.isTaskModified('deploy')).toBe(true);
      expect(component.isTaskModified('other')).toBe(false);
    });

    it('should report whether a variable is modified', () => {
      // Given
      configDirtyService.markVariableDirty('host');

      // When / Then
      expect(component.isVariableModified('host')).toBe(true);
      expect(component.isVariableModified('port')).toBe(false);
    });
  });

  describe('getOnFailStepKey', () => {
    it('should return hyphenated index key', () => {
      // Given & When & Then
      expect(component.getOnFailStepKey(2, 3)).toBe('2-3');
    });
  });

  describe('onFailStepExpanded', () => {
    it('should return false when key not in map', () => {
      // Given & When & Then
      expect(component.onFailStepExpanded(0, 1)).toBe(false);
    });

    it('should return true when key is in map as true', () => {
      // Given
      component.onFailStepExpandedMap['1-2'] = true;

      // When & Then
      expect(component.onFailStepExpanded(1, 2)).toBe(true);
    });
  });
});
