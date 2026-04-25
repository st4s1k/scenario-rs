import { ElementRef, Renderer2 } from '@angular/core';
import { TooltipComponent } from './tooltip.component';

describe('TooltipComponent', () => {
  let component: TooltipComponent;
  let mockParent: HTMLElement;
  let mockNativeElement: HTMLElement;
  let mockRenderer: jasmine.SpyObj<Renderer2>;
  let registeredListeners: { event: string; callback: Function }[];

  beforeEach(() => {
    registeredListeners = [];
    mockParent = document.createElement('div');
    mockNativeElement = document.createElement('span');
    mockParent.appendChild(mockNativeElement);

    mockRenderer = jasmine.createSpyObj('Renderer2', ['listen']);
    mockRenderer.listen.and.callFake((_target: any, event: string, callback: Function) => {
      registeredListeners.push({ event, callback });
      return () => {};
    });

    const mockElementRef = { nativeElement: mockNativeElement } as ElementRef<HTMLElement>;
    component = new TooltipComponent(mockElementRef, mockRenderer);
  });

  describe('defaults', () => {
    it('should default text to empty string', () => {
      // Given & When & Then
      expect(component.text).toBe('');
    });

    it('should default arrow to top', () => {
      // Given & When & Then
      expect(component.arrow).toBe('top');
    });
  });

  describe('host bindings', () => {
    it('should return true for arrowTop when arrow is top', () => {
      // Given
      component.arrow = 'top';

      // When & Then
      expect(component.arrowTop).toBe(true);
      expect(component.arrowLeft).toBe(false);
    });

    it('should return true for arrowLeft when arrow is left', () => {
      // Given
      component.arrow = 'left';

      // When & Then
      expect(component.arrowLeft).toBe(true);
      expect(component.arrowTop).toBe(false);
    });
  });

  describe('ngAfterViewInit', () => {
    it('should register three event listeners on the parent element', () => {
      // Given & When
      component.ngAfterViewInit();

      // Then
      expect(mockRenderer.listen).toHaveBeenCalledTimes(3);
      const events = registeredListeners.map(l => l.event);
      expect(events).toContain('mouseenter');
      expect(events).toContain('mouseleave');
      expect(events).toContain('mousedown');
    });

    it('should set visible to true on mouseenter', () => {
      // Given
      component.ngAfterViewInit();
      const mouseenterListener = registeredListeners.find(l => l.event === 'mouseenter')!;

      // When
      mouseenterListener.callback();

      // Then
      expect((component as any).visible).toBe(true);
      expect((component as any).transition).toBe(true);
    });

    it('should set visible to false on mouseleave', () => {
      // Given
      component.ngAfterViewInit();
      const mouseenterListener = registeredListeners.find(l => l.event === 'mouseenter')!;
      mouseenterListener.callback();
      const mouseleaveListener = registeredListeners.find(l => l.event === 'mouseleave')!;

      // When
      mouseleaveListener.callback();

      // Then
      expect((component as any).visible).toBe(false);
      expect((component as any).transition).toBe(true);
    });

    it('should set visible to false and transition to false on mousedown', () => {
      // Given
      component.ngAfterViewInit();
      const mouseenterListener = registeredListeners.find(l => l.event === 'mouseenter')!;
      mouseenterListener.callback();
      const mousedownListener = registeredListeners.find(l => l.event === 'mousedown')!;

      // When
      mousedownListener.callback();

      // Then
      expect((component as any).visible).toBe(false);
      expect((component as any).transition).toBe(false);
    });
  });

  describe('ngOnDestroy', () => {
    it('should call all unlisten functions', () => {
      // Given
      const unlistenSpies = [jasmine.createSpy('unlisten1'), jasmine.createSpy('unlisten2'), jasmine.createSpy('unlisten3')];
      let callIndex = 0;
      mockRenderer.listen.and.callFake(() => {
        registeredListeners.push({ event: '', callback: () => {} });
        return unlistenSpies[callIndex++];
      });
      component.ngAfterViewInit();

      // When
      component.ngOnDestroy();

      // Then
      unlistenSpies.forEach(spy => expect(spy).toHaveBeenCalledTimes(1));
    });
  });
});
