import { TestBed } from '@angular/core/testing';
import { TauriEventBridgeService } from './tauri-event-bridge.service';
import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';

describe('TauriEventBridgeService', () => {
  let service: TauriEventBridgeService;
  let tauri: TauriTestHarness;

  beforeEach(() => {
    tauri = setupTauriMock({});
    TestBed.configureTestingModule({});
    service = TestBed.inject(TauriEventBridgeService);
  });

  afterEach(() => {
    service.ngOnDestroy();
  });

  describe('connect', () => {
    it('should register a listener for the requested event', async () => {
      // Given
      service.getStream<string>('log-message');

      // When
      await new Promise(resolve => setTimeout(resolve, 0));

      // Then
      tauri.expectInvoked('plugin:event|listen');
      const calls = tauri.invokeSpy.calls.allArgs();
      const listenCall = calls.find((args: any[]) => args[0] === 'plugin:event|listen');
      expect(listenCall![1].event).toBe('log-message');
    });

    it('should return early when the service is already destroyed', async () => {
      // Given
      service.ngOnDestroy();

      // When
      await (service as any).connect('log-message');

      // Then
      tauri.expectNotInvoked('plugin:event|listen');
    });

    it('should not register the same listener twice', async () => {
      // Given
      service.getStream('log-message');
      await new Promise(resolve => setTimeout(resolve, 0));

      // When
      await (service as any).connect('log-message');

      // Then
      const listenCalls = tauri.invokeSpy.calls
        .allArgs()
        .filter((args: any[]) => args[0] === 'plugin:event|listen');
      expect(listenCalls.length).toBe(1);
    });
  });

  describe('getStream', () => {
    it('should route direct event payloads to the matching stream id', async () => {
      // Given
      const received: string[] = [];
      service.getStream<string>('log-message').subscribe(message => received.push(message));
      await new Promise(resolve => setTimeout(resolve, 0));

      // When
      tauri.emitEvent('log-message', 'hello');

      // Then
      expect(received).toEqual(['hello']);
    });

    it('should not emit to a stream with a different id', async () => {
      // Given
      const received: unknown[] = [];
      service.getStream<unknown>('execution-diff').subscribe(diff => received.push(diff));
      await new Promise(resolve => setTimeout(resolve, 0));

      // When
      tauri.emitEvent('log-message', 'hello');

      // Then
      expect(received).toEqual([]);
    });

    it('should return the same observable for the same id', () => {
      // Given
      const obs1 = service.getStream<string>('log-message');
      const obs2 = service.getStream<string>('log-message');
      const values: string[] = [];
      obs1.subscribe(value => values.push(value));
      obs2.subscribe(value => values.push(value));

      // When
      (service as any).subjects.get('log-message')?.next('shared');

      // Then
      expect(values).toEqual(['shared', 'shared']);
    });

    it('should create a new stream for an unseen id', () => {
      // Given & When
      const obs = service.getStream<number>('new-stream');

      // Then
      expect(obs).toBeTruthy();
      expect((service as any).subjects.has('new-stream')).toBeTrue();
    });

    it('should pass the event payload directly', async () => {
      // Given
      const received: unknown[] = [];
      service.getStream<unknown>('log-progress').subscribe(value => received.push(value));
      await new Promise(resolve => setTimeout(resolve, 0));

      // When
      tauri.emitEvent('log-progress', 'Progress: 50%');

      // Then
      expect(received[0]).toBe('Progress: 50%');
    });

    it('should handle array payloads for execution-diff', async () => {
      // Given
      const received: unknown[][] = [];
      service.getStream<unknown[]>('execution-diff').subscribe(value => received.push(value));
      const diffs = [{ kind: 'ExecutionStatusChanged', status: { kind: 'Running' } }];
      await new Promise(resolve => setTimeout(resolve, 0));

      // When
      tauri.emitEvent('execution-diff', diffs);

      // Then
      expect(received[0]).toEqual(diffs);
    });

    it('should register separate listeners for separate event ids', async () => {
      // Given
      service.getStream('log-message');
      service.getStream('log-progress');

      // When
      await new Promise(resolve => setTimeout(resolve, 0));

      // Then
      const listenCalls = tauri.invokeSpy.calls
        .allArgs()
        .filter((args: any[]) => args[0] === 'plugin:event|listen');
      expect(listenCalls.map((args: any[]) => args[1].event)).toEqual([
        'log-message',
        'log-progress',
      ]);
    });
  });

  describe('ngOnDestroy', () => {
    it('should call unlisten to deregister Tauri listeners', async () => {
      // Given
      service.getStream('log-message');
      service.getStream('log-progress');
      await new Promise(resolve => setTimeout(resolve, 0));

      // When
      service.ngOnDestroy();

      // Then
      tauri.expectInvoked('plugin:event|unlisten');
      const unlistenCalls = tauri.invokeSpy.calls
        .allArgs()
        .filter((args: any[]) => args[0] === 'plugin:event|unlisten');
      expect(unlistenCalls.length).toBe(2);
    });

    it('should complete all subjects on destroy', () => {
      // Given
      const subjects = (service as any).subjects;
      service.getStream('log-message');
      service.getStream('log-progress');
      const completions: string[] = [];
      service.getStream<string>('log-message').subscribe({ complete: () => completions.push('log-message') });
      service.getStream<string>('log-progress').subscribe({ complete: () => completions.push('log-progress') });

      // When
      service.ngOnDestroy();

      // Then
      expect(completions).toContain('log-message');
      expect(completions).toContain('log-progress');
      expect(subjects.size).toBe(0);
    });

    it('should not throw when unlisten is not yet set', () => {
      // Given
      const freshService = TestBed.runInInjectionContext(() => new TauriEventBridgeService());

      // When & Then
      expect(() => freshService.ngOnDestroy()).not.toThrow();
    });
  });
});
