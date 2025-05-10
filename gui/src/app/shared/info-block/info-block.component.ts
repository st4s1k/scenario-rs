import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';
import { ComponentColorVariant } from '../../models/enums';

@Component({
  selector: 'info-block',
  imports: [CommonModule],
  templateUrl: './info-block.component.html',
  styleUrls: ['./info-block.component.scss']
})
export class InfoBlockComponent {
  @Input() label: string = '';
  @Input() variant: ComponentColorVariant = 'primary';
}
