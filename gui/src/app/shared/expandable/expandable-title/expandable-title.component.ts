import { Component, Input, signal, WritableSignal } from '@angular/core';
import { ComponentColorVariant } from '../../../models/enums';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'expandable-title',
  imports: [CommonModule],
  templateUrl: './expandable-title.component.html',
  styleUrl: './expandable-title.component.scss'
})
export class ExpandableTitleComponent {

  @Input() colorIndicator: WritableSignal<ComponentColorVariant | undefined> = signal(undefined);

}
