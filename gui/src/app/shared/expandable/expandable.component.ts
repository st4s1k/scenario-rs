import { Component, Input, Output, EventEmitter, HostBinding } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ComponentSize } from '../../models/enums';

export type ExpandableSeparatorVariant = 'line' | 'gap';

@Component({
  selector: 'expandable',
  imports: [CommonModule],
  templateUrl: './expandable.component.html',
  styleUrl: './expandable.component.scss'
})
export class ExpandableComponent {

  @Input() expanded: boolean = true;
  @Input() showSeparator: boolean = false;
  @Input() separatorVariant: ExpandableSeparatorVariant = 'line';
  @Input() separatorSize: ComponentSize = 'medium';
  @Input() nested: boolean = false;

  @Output() expandedChange = new EventEmitter<boolean>();

  @HostBinding('class.nested') get isNested() { return this.nested; }
  @HostBinding('class.expanded') get isExpanded() { return this.expanded; }

  toggleExpanded(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.expanded = !this.expanded;
    this.expandedChange.emit(this.expanded);
  }
}
