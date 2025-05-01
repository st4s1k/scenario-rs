import { Component, Input, HostListener, OnChanges, Renderer2, Inject } from '@angular/core';
import { CommonModule, DOCUMENT } from '@angular/common';

interface TabConfig {
  id: string;
  title: string;
}

interface Task {
  description: string;
  error_message: string;
  task_type: string;
  command?: string;
  source_path?: string;
  destination_path?: string;
}

interface Step {
  task: Task;
  on_fail_steps: Task[];
}

@Component({
  selector: 'app-sidebar',
  imports: [CommonModule],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.scss'
})
export class SidebarComponent implements OnChanges {
  @Input() resolvedVariables: { [key: string]: string } = {};
  @Input() tasks: { [key: string]: Task } = {};
  @Input() steps: Step[] = [];

  activeTab: string = 'variables';
  sidebarWidth = 30;
  isResizing = false;
  isCollapsed = true;

  tabsConfig: TabConfig[] = [
    { id: 'steps', title: 'Steps' },
    { id: 'tasks', title: 'Tasks' },
    { id: 'variables', title: 'Variables' }
  ];

  private readonly minSidebarWidth = 40;
  private readonly titleSize = 30;
  private readonly collapseThreshold = 60;
  private readonly storageKey = 'scenario-rs-sidebar-state';
  private startX = 0;
  private startWidth = 0;
  private previousWidth = 300;

  Object = Object;

  constructor(private renderer: Renderer2, @Inject(DOCUMENT) private document: Document) {
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
        this.previousWidth = state.width || 300;
        this.activeTab = state.activeTab || 'variables';
      }
      this.sidebarWidth = this.isCollapsed ? this.titleSize : this.previousWidth;
      if (this.previousWidth < this.collapseThreshold) {
        this.previousWidth = 300;
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
        this.previousWidth = Math.max(this.collapseThreshold + 20, this.sidebarWidth);
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
      this.startX = event.clientX;
      this.startWidth = this.sidebarWidth;
      this.renderer.addClass(this.document.body, 'resizing-sidebar');
    }
    event.preventDefault();
  }

  @HostListener('window:resize')
  onResize(): void {
    if (!this.isCollapsed) {
      this.sidebarWidth = Math.min(this.sidebarWidth, window.innerWidth - 20);
    }
  }

  @HostListener('document:mousemove', ['$event'])
  onMouseMove(event: MouseEvent): void {
    if (!this.isResizing) return;

    const newWidth = this.startWidth - (event.clientX - this.startX);

    if (newWidth < this.collapseThreshold && !this.isCollapsed) {
      this.isCollapsed = true;
      this.previousWidth = Math.max(this.collapseThreshold + 20, this.startWidth);
      this.sidebarWidth = this.titleSize;
      this.isResizing = false;
      this.renderer.removeClass(this.document.body, 'resizing-sidebar');
    } else if (!this.isCollapsed) {
      this.sidebarWidth = Math.max(this.minSidebarWidth, Math.min(newWidth, window.innerWidth - 20));
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
