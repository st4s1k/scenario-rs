import { CommonModule } from '@angular/common';
import { Component, Input, ContentChild, ElementRef } from '@angular/core';

@Component({
  selector: 'info-block',
  imports: [CommonModule],
  templateUrl: './info-block.component.html',
  styleUrls: ['./info-block.component.scss']
})
export class InfoBlockComponent {
  @Input() label: string = '';
  @Input() variant: 'primary' | 'secondary' | 'error' = 'primary';
}
