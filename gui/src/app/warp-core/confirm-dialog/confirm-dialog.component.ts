import { Component, EventEmitter, HostListener, Input, Output } from '@angular/core';

@Component({
  selector: 'confirm-dialog',
  templateUrl: './confirm-dialog.component.html',
  styleUrl: './confirm-dialog.component.scss'
})
export class ConfirmDialogComponent {
  @Input() title: string = 'Confirm';
  @Input() message: string = 'Are you sure?';
  @Output() result = new EventEmitter<boolean>();

  @HostListener('click')
  onBackdropClick(): void {
    this.cancel();
  }

  confirm(): void {
    this.result.emit(true);
  }

  cancel(): void {
    this.result.emit(false);
  }
}
