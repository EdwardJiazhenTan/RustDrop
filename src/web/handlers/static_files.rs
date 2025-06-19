use axum::{
    response::{Html, IntoResponse},
};

// Serve the index.html file for the web UI
pub async fn serve_index() -> impl IntoResponse {
    // This is a simple HTML page for now
    // In a real application, you would serve a proper HTML file with CSS and JavaScript
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RustDrop - File Transfer</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            color: #333;
        }
        h1 {
            color: #2c3e50;
            text-align: center;
        }
        .container {
            display: flex;
            flex-direction: column;
            gap: 20px;
        }
        .card {
            border: 1px solid #ddd;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        }
        .file-list {
            list-style: none;
            padding: 0;
        }
        .file-item {
            display: flex;
            justify-content: space-between;
            padding: 10px;
            border-bottom: 1px solid #eee;
        }
        .file-item:last-child {
            border-bottom: none;
        }
        .upload-area {
            border: 2px dashed #3498db;
            border-radius: 8px;
            padding: 40px;
            text-align: center;
            cursor: pointer;
            transition: border-color 0.3s ease, background-color 0.3s ease;
            min-height: 120px;
            display: flex;
            align-items: center;
            justify-content: center;
            /* Better mobile touch targets */
            touch-action: manipulation;
            -webkit-touch-callout: none;
            -webkit-user-select: none;
            user-select: none;
        }
        .upload-area:hover {
            background-color: #f8f9fa;
            border-color: #2980b9;
        }
        .upload-area.active {
            background-color: #e3f2fd;
            border-color: #1976d2;
        }
        .button {
            background-color: #3498db;
            color: white;
            border: none;
            padding: 10px 15px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
        }
        .button:hover {
            background-color: #2980b9;
        }
        #file-input {
            display: none;
        }
        .device-info {
            text-align: center;
            margin-bottom: 20px;
        }
        .loading {
            text-align: center;
            padding: 20px;
        }
    </style>
</head>
<body>
    <h1>RustDrop</h1>
    <div class="device-info" id="device-info">
        <p>Loading device information...</p>
    </div>
    
    <div class="container">
        <div class="card">
            <h2>Upload Files</h2>
            <div class="upload-area" id="upload-area">
                <div>
                    <p>Select files to upload:</p>
                    <div style="display: flex; flex-wrap: wrap; gap: 10px; justify-content: center;">
                        <button type="button" class="button" id="camera-btn">📷 Camera</button>
                        <button type="button" class="button" id="photos-btn">🖼️ Photos</button>
                        <button type="button" class="button" id="files-btn">📁 All Files</button>
                    </div>
                    <p style="margin-top: 15px; font-size: 14px; color: #666;">Or drag and drop files here</p>
                </div>
                <input type="file" id="file-input-camera" accept="image/*" capture="environment">
                <input type="file" id="file-input-photos" multiple accept="image/*,video/*">
                <input type="file" id="file-input-files" multiple accept="image/*,video/*,audio/*,application/*,text/*,*/*">
            </div>
        </div>
        
        <div class="card">
            <h2>Available Files</h2>
            <div id="file-list-container">
                <p class="loading">Loading files...</p>
            </div>
        </div>
        
        <div class="card">
            <h2>Nearby Devices</h2>
            <div id="device-list-container">
                <p class="loading">Discovering devices...</p>
            </div>
            <button class="button" id="refresh-devices">Refresh Devices</button>
        </div>
    </div>

    <script>
        // Device info
        async function loadDeviceInfo() {
            try {
                const response = await fetch('/api/device');
                const device = await response.json();
                
                const deviceInfoEl = document.getElementById('device-info');
                deviceInfoEl.innerHTML = `
                    <p><strong>${device.name}</strong> (${device.os})</p>
                    <p>IP: ${device.ip}:${device.port}</p>
                `;
            } catch (error) {
                console.error('Error loading device info:', error);
            }
        }
        
        // File list
        async function loadFiles() {
            try {
                const response = await fetch('/api/files');
                const files = await response.json();
                
                const fileListContainer = document.getElementById('file-list-container');
                
                if (files.length === 0) {
                    fileListContainer.innerHTML = '<p>No files available</p>';
                    return;
                }
                
                let html = '<ul class="file-list">';
                
                files.forEach(file => {
                    const fileSize = formatFileSize(file.size);
                    html += `
                        <li class="file-item">
                            <div>
                                <strong>${file.name}</strong>
                                <div>${fileSize}</div>
                            </div>
                            <a href="/api/files/${file.id}" download="${file.name}" class="button">Download</a>
                        </li>
                    `;
                });
                
                html += '</ul>';
                fileListContainer.innerHTML = html;
            } catch (error) {
                console.error('Error loading files:', error);
                const fileListContainer = document.getElementById('file-list-container');
                fileListContainer.innerHTML = '<p>Error loading files</p>';
            }
        }
        
        // Device discovery
        async function discoverDevices() {
            try {
                const response = await fetch('/api/discover');
                const devices = await response.json();
                
                const deviceListContainer = document.getElementById('device-list-container');
                
                if (devices.length === 0) {
                    deviceListContainer.innerHTML = '<p>No devices found</p>';
                    return;
                }
                
                let html = '<ul class="file-list">';
                
                devices.forEach(device => {
                    html += `
                        <li class="file-item">
                            <div>
                                <strong>${device.name}</strong>
                                <div>${device.os} - ${device.ip}:${device.port}</div>
                            </div>
                            <a href="http://${device.ip}:${device.port}" target="_blank" class="button">Connect</a>
                        </li>
                    `;
                });
                
                html += '</ul>';
                deviceListContainer.innerHTML = html;
            } catch (error) {
                console.error('Error discovering devices:', error);
                const deviceListContainer = document.getElementById('device-list-container');
                deviceListContainer.innerHTML = '<p>Error discovering devices</p>';
            }
        }
        
        // File upload
        function setupFileUpload() {
            const uploadArea = document.getElementById('upload-area');
            const cameraInput = document.getElementById('file-input-camera');
            const photosInput = document.getElementById('file-input-photos');
            const filesInput = document.getElementById('file-input-files');
            
            // Function to reset upload area without losing event listeners
            function resetUploadArea() {
                const uploadContent = uploadArea.querySelector('div');
                uploadContent.innerHTML = `
                    <p>Select files to upload:</p>
                    <div style="display: flex; flex-wrap: wrap; gap: 10px; justify-content: center;">
                        <button type="button" class="button" id="camera-btn">📷 Camera</button>
                        <button type="button" class="button" id="photos-btn">🖼️ Photos</button>
                        <button type="button" class="button" id="files-btn">📁 All Files</button>
                    </div>
                    <p style="margin-top: 15px; font-size: 14px; color: #666;">Or drag and drop files here</p>
                `;
                
                // Re-attach button events
                setupButtons();
            }
            
            // Function to setup button event listeners
            function setupButtons() {
                const cameraBtn = document.getElementById('camera-btn');
                const photosBtn = document.getElementById('photos-btn');
                const filesBtn = document.getElementById('files-btn');
                
                if (cameraBtn) cameraBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    cameraInput.click();
                });
                
                if (photosBtn) photosBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    photosInput.click();
                });
                
                if (filesBtn) filesBtn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    filesInput.click();
                });
            }
            
            // Initial setup for buttons
            setupButtons();
            
            // Handle file selection for all inputs
            const handleFileChange = (input) => {
                return () => {
                    if (input.files.length > 0) {
                        uploadFiles(input.files, resetUploadArea);
                        // Clear the input so the same file can be selected again
                        input.value = '';
                    }
                };
            };
            
            cameraInput.addEventListener('change', handleFileChange(cameraInput));
            photosInput.addEventListener('change', handleFileChange(photosInput));
            filesInput.addEventListener('change', handleFileChange(filesInput));
            
            // Drag and drop (mainly for desktop)
            uploadArea.addEventListener('dragover', (e) => {
                e.preventDefault();
                uploadArea.classList.add('active');
            });
            
            uploadArea.addEventListener('dragleave', () => {
                uploadArea.classList.remove('active');
            });
            
            uploadArea.addEventListener('drop', (e) => {
                e.preventDefault();
                uploadArea.classList.remove('active');
                
                if (e.dataTransfer.files.length > 0) {
                    uploadFiles(e.dataTransfer.files, resetUploadArea);
                }
            });
        }
        
        async function uploadFiles(files, resetCallback) {
            const uploadArea = document.getElementById('upload-area');
            const uploadContent = uploadArea.querySelector('div');
            
            console.log('uploadFiles called with', files.length, 'files');
            
            for (const file of files) {
                try {
                    console.log('Uploading file:', file.name, 'Size:', file.size, 'Type:', file.type);
                    
                    const formData = new FormData();
                    formData.append('file', file);
                    
                    uploadContent.innerHTML = `<p>Uploading ${file.name}...</p><p>Size: ${formatFileSize(file.size)}</p>`;
                    
                    console.log('Sending POST request to /api/files');
                    const response = await fetch('/api/files', {
                        method: 'POST',
                        body: formData,
                    });
                    
                    console.log('Response status:', response.status);
                    console.log('Response headers:', response.headers);
                    
                    if (response.ok) {
                        const result = await response.json();
                        console.log('Upload successful:', result);
                        uploadContent.innerHTML = `<p>✅ Uploaded ${file.name} successfully!</p><p>Size: ${formatFileSize(result.size)}</p>`;
                        setTimeout(() => {
                            resetCallback();
                        }, 3000);
                        
                        // Reload file list
                        loadFiles();
                    } else {
                        const errorText = await response.text();
                        console.error('Upload error:', response.status, errorText);
                        uploadContent.innerHTML = `<p>❌ Error uploading ${file.name}</p><p>Status: ${response.status}</p><p>Error: ${errorText}</p>`;
                        setTimeout(() => {
                            resetCallback();
                        }, 5000);
                    }
                } catch (error) {
                    console.error('Error uploading file:', error);
                    uploadContent.innerHTML = `<p>❌ Network error uploading ${file.name}</p><p>Error: ${error.message}</p><p>Check console for details</p>`;
                    setTimeout(() => {
                        resetCallback();
                    }, 5000);
                }
            }
        }
        
        // Utility functions
        function formatFileSize(bytes) {
            if (bytes === 0) return '0 Bytes';
            
            const k = 1024;
            const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            
            return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
        }
        
        // Initialize
        document.addEventListener('DOMContentLoaded', () => {
            loadDeviceInfo();
            loadFiles();
            discoverDevices();
            setupFileUpload();
            
            // Refresh devices button
            document.getElementById('refresh-devices').addEventListener('click', () => {
                const deviceListContainer = document.getElementById('device-list-container');
                deviceListContainer.innerHTML = '<p class="loading">Discovering devices...</p>';
                discoverDevices();
            });
        });
    </script>
</body>
</html>"#;

    Html(html)
}
