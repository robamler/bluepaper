import("../pkg/index.js").catch(console.error).then(wasm => {
    function createDownloadLink(data, fileName, mimeType) {
        let blob = new Blob([data], { type: mimeType });
        let url = window.URL.createObjectURL(blob);
        let a = document.createElement('a');
        a.href = url;
        a.download = fileName;
        a.innerText = "download";
        document.body.appendChild(a);
        // TODO: revoke URL at some point: `window.URL.revokeObjectURL(url)`
    };

    function handleFileSelect(evt) {
        var file = evt.target.files[0];
        var file_name = file.name;
        file.arrayBuffer().then(buf => {
            let array = new Uint8Array(buf);

            // let latex = wasm.markdown_to_latex(array);
            // document.getElementById("latex").innerText = latex;

            let zip_file = wasm.markdown_to_zipped_latex(array, file_name);
            createDownloadLink(zip_file, "latex.zip", "application/zip");
        });
    }

    function handleFileDrop(evt) {
        evt.stopPropagation();
        evt.preventDefault();

        var file = evt.dataTransfer.files[0];
        var reader = new FileReader();

        reader.onload = (function (file_name) {
            return function (e) {
                let array = new Uint8Array(e.target.result);

                let latex = wasm.markdown_to_latex(array);
                document.getElementById("latex").innerText = latex;

                // let zip_file = wasm.markdown_to_zipped_latex(array, file_name);
                // createDownloadLink(zip_file, "latex.zip", "application/zip");
            };
        })(file.name);

        reader.readAsArrayBuffer(file);
    }

    function handleDragOver(evt) {
        evt.stopPropagation();
        evt.preventDefault();
        evt.dataTransfer.dropEffect = 'copy';
    }

    document.getElementById('files').addEventListener('change', handleFileSelect, false);

    var dropZone = document.getElementById('drop_zone');
    dropZone.addEventListener('dragover', handleDragOver, false);
    dropZone.addEventListener('drop', handleFileDrop, false);
});
