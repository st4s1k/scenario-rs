import { Component, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'expandable',
  imports: [CommonModule],
  templateUrl: './expandable.component.html',
  styleUrl: './expandable.component.scss'
})
export class ExpandableComponent {
  @Input() label: string = '';
  @Input() expanded: boolean = true;
  @Input() showSeparator: boolean = false;
  @Output() expandedChange = new EventEmitter<boolean>();

  toggleExpanded(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.expanded = !this.expanded;
    this.expandedChange.emit(this.expanded);
  }
}
