import { useState } from 'react';

interface FileUploaderProps {
  title: string;
  subtitle: string;
  onLoad: () => Promise<void>;
  loaded: boolean;
  loading: boolean;
}

export function FileUploader({
  title,
  subtitle,
  onLoad,
  loaded,
  loading,
}: FileUploaderProps) {
  const [dragOver, setDragOver] = useState(false);

  const handleDragOver = (event: React.DragEvent) => {
    event.preventDefault();
    setDragOver(true);
  };

  const handleDragLeave = () => {
    setDragOver(false);
  };

  const handleDrop = (event: React.DragEvent) => {
    event.preventDefault();
    setDragOver(false);
  };

  return (
    <div
      className={`file-uploader ${dragOver ? 'drag-over' : ''} ${loaded ? 'loaded' : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="uploader-content">
        {loaded ? (
          <>
            <div className="success-icon" aria-hidden="true">&#10003;</div>
            <h3>{title}</h3>
            <p className="success-text">Loaded successfully</p>
            <button
              className="btn-secondary btn-small"
              onClick={onLoad}
              disabled={loading}
              type="button"
            >
              Change File
            </button>
          </>
        ) : (
          <>
            <div className="upload-icon" aria-hidden="true">&#8682;</div>
            <h3>{title}</h3>
            <p>{subtitle}</p>
            <button className="btn-primary" onClick={onLoad} disabled={loading} type="button">
              {loading ? 'Loading...' : 'Select File'}
            </button>
            <p className="file-formats">Supported: WAV, MP3, FLAC, OGG, M4A, AAC</p>
          </>
        )}
      </div>
    </div>
  );
}
