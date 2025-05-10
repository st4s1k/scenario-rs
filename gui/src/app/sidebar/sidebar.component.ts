import { Component, Input, HostListener, OnChanges, Renderer2, Inject } from '@angular/core';
import { CommonModule, DOCUMENT } from '@angular/common';
import { ExpandableComponent } from '../shared/expandable/expandable.component';
import { InfoBlockComponent } from '../shared/info-block/info-block.component';
import { Step, Task } from '../app.component';

interface TabConfig {
  id: string;
  title: string;
}

@Component({
  selector: 'sidebar',
  imports: [
    CommonModule,
    ExpandableComponent,
    InfoBlockComponent
  ],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.scss'
})
export class SidebarComponent implements OnChanges {

  private readonly titleSize = 1.5;
  private readonly collapseThreshold = this.titleSize + 0.625;
  private readonly minSidebarWidth = this.collapseThreshold + 0.0625;
  private readonly storageKey = 'scenario-rs-sidebar-state';
  private readonly htmlFontSize;

  @Input() resolvedVariables: { [key: string]: string } = {};
  @Input() tasks: { [key: string]: Task } = {};
  @Input() steps: Step[] = [];

  activeTab: string = 'variables';
  sidebarWidth = this.titleSize;
  isResizing = false;
  isCollapsed = true;

  taskExpandedMap: { [key: string]: boolean } = {};
  stepExpandedMap: { [index: number]: boolean } = {};
  onFailExpandedMap: { [index: number]: boolean } = {};
  onFailStepExpandedMap: { [key: string]: boolean } = {};

  getOnFailStepKey(parentIndex: number, onFailIndex: number): string {
    return `${parentIndex}-${onFailIndex}`;
  }

  onFailStepExpanded(parentIndex: number, onFailIndex: number): boolean {
    const key = this.getOnFailStepKey(parentIndex, onFailIndex);
    return this.onFailStepExpandedMap[key] || false;
  }

  tabsConfig: TabConfig[] = [
    { id: 'steps', title: 'Steps' },
    { id: 'tasks', title: 'Tasks' },
    { id: 'variables', title: 'Variables' }
  ];

  private startX = 0;
  private startWidth = 0;
  private previousWidth = 18.75;

  Object = Object;

  constructor(private renderer: Renderer2, @Inject(DOCUMENT) private document: Document) {
    this.htmlFontSize = parseFloat(getComputedStyle(this.document.documentElement).fontSize);
    this.loadSavedState();
  }

  ngOnChanges(): void {
  }

  private loadSavedState(): void {
    try {
      const savedState = localStorage.getItem(this.storageKey);
      if (savedState) {
        const state = JSON.parse(savedState);
        this.isCollapsed = state.collapsed !== undefined ? state.collapsed : true;
        this.previousWidth = state.width || 18.75;
        this.activeTab = state.activeTab || 'variables';
      }
      this.sidebarWidth = this.isCollapsed ? this.titleSize : this.previousWidth;
      if (this.previousWidth < this.collapseThreshold) {
        this.previousWidth = 18.75;
      }
    } catch (e) {
      this.isCollapsed = true;
      this.sidebarWidth = this.titleSize;
    }
  }

  private saveState(): void {
    try {
      const state = {
        width: this.isCollapsed ? this.previousWidth : this.sidebarWidth,
        collapsed: this.isCollapsed,
        activeTab: this.activeTab
      };
      localStorage.setItem(this.storageKey, JSON.stringify(state));
    } catch (e) {
      console.warn('Error saving sidebar state:', e);
    }
  }

  isTabActive(tabId: string): boolean {
    return !this.isCollapsed && this.activeTab === tabId;
  }

  toggleTab(tabId: string): void {
    if (this.activeTab === tabId) {
      this.isCollapsed = !this.isCollapsed;
      if (this.isCollapsed) {
        this.previousWidth = Math.max(this.collapseThreshold + 1.25, this.sidebarWidth);
        this.sidebarWidth = this.titleSize;
      } else {
        this.sidebarWidth = this.previousWidth;
      }
    } else {
      if (this.isCollapsed) {
        this.isCollapsed = false;
        this.sidebarWidth = this.previousWidth;
      }
      this.activeTab = tabId;
    }
    this.saveState();
  }

  startResize(event: MouseEvent): void {
    if (!this.isCollapsed) {
      this.isResizing = true;
      this.startX = event.clientX / this.htmlFontSize;
      this.startWidth = this.sidebarWidth;
      this.renderer.addClass(this.document.body, 'resizing-sidebar');
    }
    event.preventDefault();
  }

  @HostListener('window:resize')
  onResize(): void {
    if (!this.isCollapsed) {
      this.sidebarWidth = Math.min(this.sidebarWidth, window.innerWidth - 1.25);
    }
  }

  @HostListener('document:mousemove', ['$event'])
  onMouseMove(event: MouseEvent): void {
    if (!this.isResizing) return;

    const clientX = event.clientX / this.htmlFontSize;
    const newWidth = this.startWidth - (clientX - this.startX);

    if (newWidth < this.collapseThreshold && !this.isCollapsed) {
      this.isCollapsed = true;
      this.previousWidth = Math.max(this.collapseThreshold + 1.25, this.startWidth);
      this.sidebarWidth = this.titleSize;
      this.isResizing = false;
      this.renderer.removeClass(this.document.body, 'resizing-sidebar');
    } else if (!this.isCollapsed) {
      this.sidebarWidth = Math.max(this.minSidebarWidth, Math.min(newWidth, window.innerWidth - 1.25));
    }

    event.preventDefault();
  }

  @HostListener('document:mouseup')
  onMouseUp(): void {
    if (this.isResizing) {
      this.isResizing = false;
      this.renderer.removeClass(this.document.body, 'resizing-sidebar');
      this.saveState();
    }
  }

  @HostListener('document:keydown', ['$event'])
  handleKeyboardEvent(event: KeyboardEvent): void {
    // Toggle sidebar with Alt+S
    if (event.altKey && event.key === 's') {
      this.isCollapsed = !this.isCollapsed;
      this.sidebarWidth = this.isCollapsed ? this.titleSize : this.previousWidth;
      this.saveState();
      event.preventDefault();
    }

    // Switch tabs with Alt+1, Alt+2, Alt+3 etc.
    if (event.altKey && !isNaN(Number(event.key))) {
      const tabIndex = Number(event.key) - 1;
      const tabIds = this.tabsConfig.map(tab => tab.id);
      if (tabIndex >= 0 && tabIndex < tabIds.length) {
        const tabId = tabIds[tabIndex];
        this.toggleTab(tabId);
        event.preventDefault();
      }
    }
  }
}
