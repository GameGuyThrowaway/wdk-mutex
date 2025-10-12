# Script for auto-generating docs which the CI will take care of

# REMEMBER: To un-comment:
# [package.metadata.wdk.driver-model]
# driver-type = "WDM"
#
# in the toml..

cargo check
cargo doc --no-deps --lib

Remove-Item -Recurse -Force ./docs
'<meta http-equiv="refresh" content="0; url=wdk_mutex">' | Out-File -Encoding ascii -FilePath ./target/doc/index.html -Force

mkdir docs
Copy-Item -Path ./target/doc/* -Destination ./docs -Recurse -Force -Container

Write-Output "All done, don't forget to comment out wdm / kmdf in the toml!"