import { Renderer2 } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { SidebarComponent } from './sidebar.component';

const DEFAULT_TABS = [
  { id: 'steps', title: 'Steps' },
  { id: 'tasks', title: 'Tasks' },
  { id: 'variables', title: 'Variables' },
];

describe('SidebarComponent', () => {
  let component: SidebarComponent;
  let mockRenderer: jasmine.SpyObj<Renderer2>;
  let mockDocument: Document;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    mockRenderer = jasmine.createSpyObj('Renderer2', ['addClass', 'removeClass']);
    mockDocument = document;
    component = TestBed.runInInjectionContext(() => new SidebarComponent(mockRenderer, mockDocument));
    component.tabs = DEFAULT_TABS;
    component.ngOnInit();
  });

  describe('ngOnInit', () => {
    it('should set activeTab to last tab by default', () => {
      // Given & When (ngOnInit already called in beforeEach)

      // Then
      expect(component.activeTab()).toBe('variables');
    });

    it('should not overwrite a pre-set activeTab', () => {
      // Given
      const freshComponent = TestBed.runInInjectionContext(() => new SidebarComponent(mockRenderer, mockDocument));
      freshComponent.tabs = DEFAULT_TABS;
      freshComponent.activeTab.set('steps');

      // When
      freshComponent.ngOnInit();

      // Then
      expect(freshComponent.activeTab()).toBe('steps');
    });
  });

  describe('isTabActive', () => {
    it('should return false when collapsed', () => {
      // Given
      component.isCollapsed = true;

      // When & Then
      expect(component.isTabActive('variables')).toBe(false);
    });

    it('should return true when not collapsed and tab matches', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('variables');

      // When & Then
      expect(component.isTabActive('variables')).toBe(true);
    });

    it('should return false when not collapsed but tab does not match', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('steps');

      // When & Then
      expect(component.isTabActive('variables')).toBe(false);
    });
  });

  describe('toggleTab', () => {
    it('should collapse when toggling the active tab', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('steps');
      component.sidebarWidth = 15;

      // When
      component.toggleTab('steps');

      // Then
      expect(component.isCollapsed).toBe(true);
    });

    it('should expand when toggling the active tab from collapsed', () => {
      // Given
      component.isCollapsed = true;
      component.activeTab.set('steps');

      // When
      component.toggleTab('steps');

      // Then
      expect(component.isCollapsed).toBe(false);
    });

    it('should switch tab and expand when selecting a different tab while collapsed', () => {
      // Given
      component.isCollapsed = true;
      component.activeTab.set('steps');

      // When
      component.toggleTab('variables');

      // Then
      expect(component.isCollapsed).toBe(false);
      expect(component.activeTab()).toBe('variables');
    });

    it('should switch tab without collapsing when selecting a different tab while expanded', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('steps');

      // When
      component.toggleTab('variables');

      // Then
      expect(component.isCollapsed).toBe(false);
      expect(component.activeTab()).toBe('variables');
    });

    it('should preserve previous width when collapsing', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('steps');
      component.sidebarWidth = 20;

      // When
      component.toggleTab('steps');

      // Then
      expect(component.isCollapsed).toBe(true);
      component.toggleTab('steps');
      expect(component.sidebarWidth).toBeGreaterThan(0);
    });
  });

  describe('startResize', () => {
    it('should set isResizing when not collapsed', () => {
      // Given
      component.isCollapsed = false;
      component.sidebarWidth = 15;
      const event = new MouseEvent('mousedown', { clientX: 500 });
      spyOn(event, 'preventDefault');

      // When
      component.startResize(event);

      // Then
      expect(component.isResizing).toBe(true);
      expect(event.preventDefault).toHaveBeenCalled();
    });

    it('should not set isResizing when collapsed', () => {
      // Given
      component.isCollapsed = true;
      const event = new MouseEvent('mousedown', { clientX: 500 });
      spyOn(event, 'preventDefault');

      // When
      component.startResize(event);

      // Then
      expect(component.isResizing).toBe(false);
    });

    it('should add resizing-sidebar class to body', () => {
      // Given
      component.isCollapsed = false;
      component.sidebarWidth = 15;
      const event = new MouseEvent('mousedown', { clientX: 500 });

      // When
      component.startResize(event);

      // Then
      expect(mockRenderer.addClass).toHaveBeenCalledWith(mockDocument.body, 'resizing-sidebar');
    });
  });

  describe('onMouseUp', () => {
    it('should stop resizing and remove class', () => {
      // Given
      component.isResizing = true;

      // When
      component.onMouseUp();

      // Then
      expect(component.isResizing).toBe(false);
      expect(mockRenderer.removeClass).toHaveBeenCalledWith(mockDocument.body, 'resizing-sidebar');
    });

    it('should do nothing when not resizing', () => {
      // Given
      component.isResizing = false;

      // When
      component.onMouseUp();

      // Then
      expect(mockRenderer.removeClass).not.toHaveBeenCalled();
    });
  });

  describe('onResize', () => {
    it('should constrain width to window when not collapsed', () => {
      // Given
      component.isCollapsed = false;
      component.sidebarWidth = 99999;

      // When
      component.onResize();

      // Then
      expect(component.sidebarWidth).toBeLessThanOrEqual(window.innerWidth - 1.25);
    });

    it('should not change width when collapsed', () => {
      // Given
      component.isCollapsed = true;
      const originalWidth = component.sidebarWidth;

      // When
      component.onResize();

      // Then
      expect(component.sidebarWidth).toBe(originalWidth);
    });
  });

  describe('handleKeyboardEvent', () => {
    it('should toggle collapsed state on Alt+S', () => {
      // Given
      component.isCollapsed = false;
      const event = new KeyboardEvent('keydown', { altKey: true, key: 's' });
      spyOn(event, 'preventDefault');

      // When
      component.handleKeyboardEvent(event);

      // Then
      expect(component.isCollapsed).toBe(true);
      expect(event.preventDefault).toHaveBeenCalled();
    });

    it('should expand on Alt+S when collapsed', () => {
      // Given
      component.isCollapsed = true;
      const event = new KeyboardEvent('keydown', { altKey: true, key: 's' });

      // When
      component.handleKeyboardEvent(event);

      // Then
      expect(component.isCollapsed).toBe(false);
    });

    it('should switch to tab on Alt+1', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('variables');
      const event = new KeyboardEvent('keydown', { altKey: true, key: '1' });
      spyOn(event, 'preventDefault');

      // When
      component.handleKeyboardEvent(event);

      // Then
      expect(event.preventDefault).toHaveBeenCalled();
    });

    it('should ignore Alt+number for out-of-range index', () => {
      // Given
      component.isCollapsed = false;
      const event = new KeyboardEvent('keydown', { altKey: true, key: '9' });
      spyOn(event, 'preventDefault');

      // When
      component.handleKeyboardEvent(event);

      // Then
      expect(event.preventDefault).not.toHaveBeenCalled();
    });

    it('should ignore non-Alt key events', () => {
      // Given
      const originalCollapsed = component.isCollapsed;
      const event = new KeyboardEvent('keydown', { altKey: false, key: 's' });

      // When
      component.handleKeyboardEvent(event);

      // Then
      expect(component.isCollapsed).toBe(originalCollapsed);
    });

    it('should ignore Alt+non-number key', () => {
      // Given
      component.isCollapsed = false;
      component.activeTab.set('variables');
      const event = new KeyboardEvent('keydown', { altKey: true, key: 'a' });
      spyOn(event, 'preventDefault');

      // When
      component.handleKeyboardEvent(event);

      // Then
      expect(event.preventDefault).not.toHaveBeenCalled();
      expect(component.activeTab()).toBe('variables');
    });
  });

  describe('onMouseMove', () => {
    it('should collapse sidebar when dragged past threshold', () => {
      // Given
      component.isCollapsed = false;
      component.isResizing = true;
      component.sidebarWidth = 15;
      (component as any).startWidth = 15;
      (component as any).startX = 500 / (component as any).htmlFontSize;
      const collapseThreshold = (component as any).collapseThreshold;
      const clientX = (500 + 15 * (component as any).htmlFontSize - collapseThreshold * (component as any).htmlFontSize + (component as any).htmlFontSize);
      const event = new MouseEvent('mousemove', { clientX });
      spyOn(event, 'preventDefault');

      // When
      component.onMouseMove(event);

      // Then
      expect(component.isCollapsed).toBe(true);
      expect(component.isResizing).toBe(false);
      expect(mockRenderer.removeClass).toHaveBeenCalledWith(mockDocument.body, 'resizing-sidebar');
    });

    it('should clamp width within bounds when not collapsing', () => {
      // Given
      component.isCollapsed = false;
      component.isResizing = true;
      (component as any).startWidth = 20;
      (component as any).startX = 500 / (component as any).htmlFontSize;
      const clientX = 490;
      const event = new MouseEvent('mousemove', { clientX });
      spyOn(event, 'preventDefault');

      // When
      component.onMouseMove(event);

      // Then
      const minWidth = (component as any).minSidebarWidth;
      expect(component.sidebarWidth).toBeGreaterThanOrEqual(minWidth);
      expect(component.sidebarWidth).toBeLessThanOrEqual(window.innerWidth - 1.25);
      expect(event.preventDefault).toHaveBeenCalled();
    });

    it('should do nothing when not resizing', () => {
      // Given
      component.isResizing = false;
      const originalWidth = component.sidebarWidth;
      const event = new MouseEvent('mousemove', { clientX: 100 });

      // When
      component.onMouseMove(event);

      // Then
      expect(component.sidebarWidth).toBe(originalWidth);
    });
  });
});
