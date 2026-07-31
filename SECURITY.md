# Security Policy

## Supported Versions

LocalScribe is pre-1.0 software. Security fixes are handled on the main development line until a release policy is established.

## Reporting a Vulnerability

Please do not open a public issue for a suspected vulnerability.

Use GitHub private vulnerability reporting if it is enabled for the repository. If it is not enabled, contact the repository maintainer through the repository owner's GitHub profile and include only the minimum information needed to start triage.

Helpful details:

- Affected commit or release.
- Operating system version.
- Clear reproduction steps.
- Expected and actual impact.
- Whether the issue involves local files, model downloads, FFmpeg processing, Tauri command permissions, or exported transcript files.

Do not send private media files unless a maintainer explicitly asks for a safe reproduction artifact.

## Security Expectations

LocalScribe should:

- Keep user media files local.
- Avoid telemetry by default.
- Keep Tauri permissions narrow.
- Validate downloaded model checksums when configured.
- Avoid committing secrets, local settings, build artifacts, or downloaded models.

## Out of Scope

The following are usually not treated as vulnerabilities by themselves:

- Transcription quality issues.
- Model hallucinations or inaccurate speaker labels.
- Problems caused by modified local builds.
- Issues in third-party models, FFmpeg, or operating system components unless LocalScribe makes them exploitable in a new way.
