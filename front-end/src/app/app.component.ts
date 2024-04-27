import { Component } from '@angular/core';
import { UploadFileComponent } from './upload-file/upload-file.component';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [UploadFileComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent {

}
