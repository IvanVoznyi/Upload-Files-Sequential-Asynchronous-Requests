import { Component } from '@angular/core';

@Component({
  selector: 'app-upload-file',
  standalone: true,
  templateUrl: './upload-file.component.html',
  styleUrl: './upload-file.component.scss'
})
export class UploadFileComponent {
  files: string[] = [];
  abortControllers: { [fileName: string]: AbortController } = {};
  progressMap: { [fileName: string]: number } = {};

  onDragOver(event: DragEvent) {
    event.preventDefault();
    (event.target as HTMLDivElement).classList.add('drag-over');
  }

  onDrop(event: DragEvent) {
    event.preventDefault();
    (event.target as HTMLDivElement).classList.remove('drag-over');
    if (event.dataTransfer) {
      for (const file of Array.from(event.dataTransfer.files)) {
        this.files.push(file.name)
        this.uploadFile(file);
      }
    }
  }

  onDragLeave(event: DragEvent) {
    event.preventDefault();
    (event.target as HTMLDivElement).classList.remove('drag-over');
  }

  onCancelUploadFile(fileName: string) {
    if (this.abortControllers[fileName]) {
      this.abortControllers[fileName].abort();
      delete this.abortControllers[fileName];
    }
  }

  async* chunkFile(file: File, chunkSize: number) {
    let index = 0;
    let totalUploaded = 0
    const totalChunks = Math.ceil(file.size / chunkSize);
    const fileReader = new FileReader();

    while (index < totalChunks) {
      const start = index * chunkSize;
      const end = Math.min(start + chunkSize, file.size);
      const chunk = file.slice(start, end);

      const buffer = await new Promise<ArrayBuffer>((resolve, reject) => {
        fileReader.onload = (e) => {
          if (e.target && e.target.result instanceof ArrayBuffer) {
            resolve(e.target.result)
          }
        };
        fileReader.onerror = () => reject('Error reading file chunk');
        fileReader.readAsArrayBuffer(chunk);
      });

      totalUploaded += buffer.byteLength
      yield { chunk: buffer, index: index + 1, totalChunks, file, totalUploaded };
      index++;
    }
  }

  async uploadFile(file: File) {
    const chunkSize = 5 * 1024 * 1024; // 5 MB chunk size
    const generator = this.chunkFile(file, chunkSize);

    let result = await generator.next();
    while (!result.done) {
      const chunk = result.value;

      try {
        const uploadResult = await this.uploadChunk(chunk);
        if (uploadResult && uploadResult.ok) {
          this.progressMap[file.name] = Math.round((chunk.totalUploaded / file.size) * 100)
        }
      } catch (error) {
        console.error(`Chunk ${chunk.index} upload error:`, error);
        break;
      }

      result = await generator.next();
    }
  }

  uploadChunk({ chunk, file, index, totalChunks }: {
    chunk: ArrayBuffer;
    index: number;
    totalChunks: number;
    file: File;
    totalUploaded: number
  }): Promise<Response> {
    const formData = new FormData();
    formData.append('chunk', new Blob([chunk]));
    this.abortControllers[file.name] = new AbortController();
    return fetch('http://localhost:8080/upload', {
      method: 'POST',
      body: formData,
      headers: {
        'X-File-Name': file.name,
        'X-File-Size': file.size.toString(),
        'X-Chunk-Index': index.toString(),
        'X-Total-Chunks': totalChunks.toString(),
      },
      signal: this.abortControllers[file.name].signal
    })
  }
}
