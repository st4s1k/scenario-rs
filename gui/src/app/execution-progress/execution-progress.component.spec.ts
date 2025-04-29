import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ExecutionProgressComponent } from './execution-progress.component';

describe('ExecutionProgressComponent', () => {
  let component: ExecutionProgressComponent;
  let fixture: ComponentFixture<ExecutionProgressComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ExecutionProgressComponent]
    })
      .compileComponents();

    fixture = TestBed.createComponent(ExecutionProgressComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
