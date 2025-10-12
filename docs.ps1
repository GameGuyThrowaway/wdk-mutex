# Script for auto-generating docs which the CI will take care of
cargo doc --no-deps --lib --target-dir docs2

Remove-Item -Recurse -Force ./docs
'<meta http-equiv="refresh" content="0; url=wdk_mutex">' | Out-File -Encoding ascii -FilePath ./docs2/doc/index.html -Force

mkdir docs
Copy-Item -Path ./docs2/doc/* -Destination ./docs -Recurse -Force -Container

Remove-Item -Recurse -Force ./docs2

Write-Output "All done, don't forget to comment out wdm / kmdf in the toml!"