import { signal } from '@angular/core';
import { ExpandableTitleComponent } from './expandable-title.component';

describe('ExpandableTitleComponent', () => {
  let component: ExpandableTitleComponent;

  beforeEach(() => {
    component = new ExpandableTitleComponent();
  });

  describe('defaults', () => {
    it('should default colorIndicator to undefined signal', () => {
      // Given & When & Then
      expect(component.colorIndicator()).toBeUndefined();
    });
  });

  describe('colorIndicator', () => {
    it('should accept an external writable signal', () => {
      // Given
      const colorSignal = signal<any>('blue');

      // When
      component.colorIndicator = colorSignal;

      // Then
      expect(component.colorIndicator()).toBe('blue');
    });

    it('should reflect signal updates', () => {
      // Given
      const colorSignal = signal<any>('green');
      component.colorIndicator = colorSignal;

      // When
      colorSignal.set('red');

      // Then
      expect(component.colorIndicator()).toBe('red');
    });
  });
});
