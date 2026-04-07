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
    // Provide basic dimensions for scroll calculations
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
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 290, configurable: true }); // distance = 10, < 32

      // When
      (directive as any).onScroll();

      // Then
      expect((directive as any).autoScrollEnabled).toBe(true);
    });

    it('should disable auto-scroll when far from bottom', () => {
      // Given
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, configurable: true });
      Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, configurable: true });
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 200, configurable: true }); // distance = 100, > 32

      // When
      (directive as any).onScroll();

      // Then
      expect((directive as any).autoScrollEnabled).toBe(false);
    });

    it('should enable auto-scroll at exactly 32px threshold', () => {
      // Given
      Object.defineProperty(mockTextArea, 'scrollHeight', { value: 500, configurable: true });
      Object.defineProperty(mockTextArea, 'clientHeight', { value: 200, configurable: true });
      Object.defineProperty(mockTextArea, 'scrollTop', { value: 269, configurable: true }); // distance = 31, < 32

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

      // Allow RAF to fire
      requestAnimationFrame(() => {
        expect((directive as any).pending).toBe(false);
        done();
      });
    });
  });
});
