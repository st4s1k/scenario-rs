import { Component, Input, HostListener, Renderer2, Inject, signal } from '@angular/core';
import { DOCUMENT } from '@angular/common';

export interface TabConfig {
  id: string;
  title: string;
}

@Component({
  selector: 'sidebar',
  imports: [],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.scss'
})
export class SidebarComponent {

  private readonly titleSize = 1.5;
  private readonly collapseThreshold = this.titleSize + 0.625;
  private readonly minSidebarWidth = this.collapseThreshold + 0.0625;
  private readonly htmlFontSize;

  @Input() tabs: TabConfig[] = [];

  activeTab = signal('');
  sidebarWidth = this.titleSize;
  isResizing = false;
  isCollapsed = true;

  private startX = 0;
  private startWidth = 0;
  private previousWidth = 18.75;

  constructor(private renderer: Renderer2, @Inject(DOCUMENT) private document: Document) {
    this.htmlFontSize = parseFloat(getComputedStyle(this.document.documentElement).fontSize);
  }

  ngOnInit(): void {
    if (this.tabs.length > 0 && !this.activeTab()) {
      this.activeTab.set(this.tabs[this.tabs.length - 1].id);
    }
  }

  isTabActive(tabId: string): boolean {
    return !this.isCollapsed && this.activeTab() === tabId;
  }

  toggleTab(tabId: string): void {
    if (this.activeTab() === tabId) {
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
      this.activeTab.set(tabId);
    }
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
    }
  }

  @HostListener('document:keydown', ['$event'])
  handleKeyboardEvent(event: KeyboardEvent): void {
    if (event.altKey && event.key === 's') {
      this.isCollapsed = !this.isCollapsed;
      this.sidebarWidth = this.isCollapsed ? this.titleSize : this.previousWidth;
      event.preventDefault();
    }

    if (event.altKey && !isNaN(Number(event.key))) {
      const tabIndex = Number(event.key) - 1;
      const tabIds = this.tabs.map(tab => tab.id);
      if (tabIndex >= 0 && tabIndex < tabIds.length) {
        this.toggleTab(tabIds[tabIndex]);
        event.preventDefault();
      }
    }
  }
}
