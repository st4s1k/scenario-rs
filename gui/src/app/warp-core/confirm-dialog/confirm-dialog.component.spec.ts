import { ConfirmDialogComponent } from './confirm-dialog.component';

describe('ConfirmDialogComponent', () => {
  let component: ConfirmDialogComponent;

  beforeEach(() => {
    component = new ConfirmDialogComponent();
  });

  it('should have default title and message', () => {
    expect(component.title).toBe('Confirm');
    expect(component.message).toBe('Are you sure?');
  });

  describe('confirm', () => {
    it('should emit true', () => {
      // Given
      spyOn(component.result, 'emit');

      // When
      component.confirm();

      // Then
      expect(component.result.emit).toHaveBeenCalledWith(true);
    });
  });

  describe('cancel', () => {
    it('should emit false', () => {
      // Given
      spyOn(component.result, 'emit');

      // When
      component.cancel();

      // Then
      expect(component.result.emit).toHaveBeenCalledWith(false);
    });
  });

  describe('onBackdropClick', () => {
    it('should cancel on backdrop click', () => {
      // Given
      spyOn(component.result, 'emit');

      // When
      component.onBackdropClick();

      // Then
      expect(component.result.emit).toHaveBeenCalledWith(false);
    });
  });
});
