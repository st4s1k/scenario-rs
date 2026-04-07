import { InfoBlockComponent } from './info-block.component';

describe('InfoBlockComponent', () => {
  let component: InfoBlockComponent;

  beforeEach(() => {
    component = new InfoBlockComponent();
  });

  describe('defaults', () => {
    it('should default label to empty string', () => {
      // Given & When & Then
      expect(component.label).toBe('');
    });

    it('should default variant to primary', () => {
      // Given & When & Then
      expect(component.variant).toBe('primary');
    });
  });

  describe('inputs', () => {
    it('should accept a label', () => {
      // Given & When
      component.label = 'Status: OK';

      // Then
      expect(component.label).toBe('Status: OK');
    });

    it('should accept a variant', () => {
      // Given & When
      component.variant = 'red';

      // Then
      expect(component.variant).toBe('red');
    });
  });
});
