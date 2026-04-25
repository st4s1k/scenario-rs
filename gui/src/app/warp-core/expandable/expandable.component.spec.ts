import { ExpandableComponent } from './expandable.component';

describe('ExpandableComponent', () => {
  let component: ExpandableComponent;

  beforeEach(() => {
    component = new ExpandableComponent();
  });

  describe('defaults', () => {
    it('should default expanded to true', () => {
      // Given & When & Then
      expect(component.expanded).toBe(true);
    });

    it('should default showSeparator to false', () => {
      // Given & When & Then
      expect(component.showSeparator).toBe(false);
    });

    it('should default separatorVariant to line', () => {
      // Given & When & Then
      expect(component.separatorVariant).toBe('line');
    });

    it('should default separatorSize to medium', () => {
      // Given & When & Then
      expect(component.separatorSize).toBe('medium');
    });

    it('should default nested to false', () => {
      // Given & When & Then
      expect(component.nested).toBe(false);
    });
  });

  describe('host bindings', () => {
    it('should return nested state via isNested', () => {
      // Given
      component.nested = true;

      // When & Then
      expect(component.isNested).toBe(true);
    });

    it('should return expanded state via isExpanded', () => {
      // Given
      component.expanded = false;

      // When & Then
      expect(component.isExpanded).toBe(false);
    });
  });

  describe('toggleExpanded', () => {
    it('should toggle expanded from true to false', () => {
      // Given
      component.expanded = true;
      const event = new MouseEvent('click');
      spyOn(event, 'preventDefault');
      spyOn(event, 'stopPropagation');

      // When
      component.toggleExpanded(event);

      // Then
      expect(component.expanded).toBe(false);
    });

    it('should toggle expanded from false to true', () => {
      // Given
      component.expanded = false;
      const event = new MouseEvent('click');

      // When
      component.toggleExpanded(event);

      // Then
      expect(component.expanded).toBe(true);
    });

    it('should emit expandedChange with the new value', () => {
      // Given
      component.expanded = true;
      const event = new MouseEvent('click');
      let emittedValue: boolean | undefined;
      component.expandedChange.subscribe((val: boolean) => emittedValue = val);

      // When
      component.toggleExpanded(event);

      // Then
      expect(emittedValue).toBe(false);
    });

    it('should call preventDefault and stopPropagation', () => {
      // Given
      const event = new MouseEvent('click');
      spyOn(event, 'preventDefault');
      spyOn(event, 'stopPropagation');

      // When
      component.toggleExpanded(event);

      // Then
      expect(event.preventDefault).toHaveBeenCalled();
      expect(event.stopPropagation).toHaveBeenCalled();
    });
  });
});
