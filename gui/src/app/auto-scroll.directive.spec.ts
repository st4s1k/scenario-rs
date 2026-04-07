import { ElementRef, NgZone } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { AutoScrollDirective } from './auto-scroll.directive';
import { signal } from '@angular/core';

describe('AutoScrollDirective', () => {
  let directive: AutoScrollDirective;
  let mockTextArea: HTMLTextAreaElement;
  let ngZone: NgZone;

  beforeEach(() => {
    mockTextArea = document.createElement('textarea');
    Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, writable: true, configurable: true });
    Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, writable: true, configurable: true });
    Object.defineProperty(mockTextArea, 'isConnected', { value: true, writable: true, configurable: true });

    ngZone = TestBed.inject(NgZone);

    const elementRef = new ElementRef(mockTextArea);
    directive = TestBed.runInInjectionContext(() => new AutoScrollDirective(elementRef, ngZone));
  });

  describe('onScroll', () => {
    it('should enable auto-scroll when near bottom', () => {
      // Given
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, configurable: true });
      Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, configurable: true });
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 290, configurable: true });

      // When
      (directive as any).onScroll();

      // Then
      expect((directive as any).autoScrollEnabled).toBe(true);
    });

    it('should disable auto-scroll when far from bottom', () => {
      // Given
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, configurable: true });
      Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, configurable: true });
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 200, configurable: true });

      // When
      (directive as any).onScroll();

      // Then
      expect((directive as any).autoScrollEnabled).toBe(false);
    });

    it('should enable auto-scroll at exactly 32px threshold', () => {
      // Given
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, configurable: true });
      Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, configurable: true });
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 269, configurable: true });

      // When
      (directive as any).onScroll();

      // Then
      expect((directive as any).autoScrollEnabled).toBe(true);
    });
  });

  describe('scheduleScroll', () => {
    it('should not schedule when autoScrollEnabled is false', () => {
      // Given
      (directive as any).autoScrollEnabled = false;
      spyOn(window, 'requestAnimationFrame');

      // When
      (directive as any).scheduleScroll();

      // Then
      expect(window.requestAnimationFrame).not.toHaveBeenCalled();
    });

    it('should not schedule when already pending', () => {
      // Given
      (directive as any).autoScrollEnabled = true;
      (directive as any).pending = true;
      spyOn(window, 'requestAnimationFrame');

      // When
      (directive as any).scheduleScroll();

      // Then
      expect(window.requestAnimationFrame).not.toHaveBeenCalled();
    });

    it('should schedule when enabled and not pending', (done) => {
      // Given
      (directive as any).autoScrollEnabled = true;
      (directive as any).pending = false;

      // When
      (directive as any).scheduleScroll();

      // Then
      expect((directive as any).pending).toBe(true);

      requestAnimationFrame(() => {
        expect((directive as any).pending).toBe(false);
        done();
      });
    });

    it('should set scrollTop when element is connected', (done) => {
      // Given
      (directive as any).autoScrollEnabled = true;
      (directive as any).pending = false;
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 1000, configurable: true });
      Object.defineProperty(mockTextArea, 'isConnected', { value: true, configurable: true });

      // When
      (directive as any).scheduleScroll();

      // Then
      requestAnimationFrame(() => {
        expect(mockTextArea.scrollTop).toBe(1000);
        done();
      });
    });

    it('should not set scrollTop when element is disconnected', (done) => {
      // Given
      (directive as any).autoScrollEnabled = true;
      (directive as any).pending = false;
      mockTextArea.scrollTop = 0;
      Object.defineProperty(mockTextArea, 'isConnected', { value: false, configurable: true });

      // When
      (directive as any).scheduleScroll();

      // Then
      requestAnimationFrame(() => {
        expect(mockTextArea.scrollTop).toBe(0);
        done();
      });
    });
  });

  describe('valueChangedSignal effect', () => {
    it('should trigger scheduleScroll when signal value changes', () => {
      // Given
      const testSignal = signal('initial');
      (directive as any).valueChangedSignal = testSignal;
      (directive as any).autoScrollEnabled = true;
      (directive as any).pending = false;

      // When
      TestBed.flushEffects();
      testSignal.set('updated');
      TestBed.flushEffects();

      // Then
      expect((directive as any).pending).toBe(true);
    });
  });

  describe('scroll listener', () => {
    it('should update autoScrollEnabled when scroll event fires', () => {
      // Given
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, configurable: true });
      Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, configurable: true });
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 100, configurable: true });

      // When
      mockTextArea.dispatchEvent(new Event('scroll'));

      // Then
      expect((directive as any).autoScrollEnabled).toBe(false);
    });
  });
});
