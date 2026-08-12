# LocalScribe launch kit

This kit is ready for the first signed and notarized Apple Silicon release.
Do not start the public launch while the release is source-only or the DMG has
not passed the clean-Mac smoke test.

## Links

- Landing page: https://ukolov-dev.github.io/LocalScribe/
- Repository: https://github.com/ukolov-dev/LocalScribe
- Release: https://github.com/ukolov-dev/LocalScribe/releases/tag/v0.1.2
- Roadmap: https://github.com/ukolov-dev/LocalScribe/blob/main/ROADMAP.md

## Positioning

**One line:** Private, offline audio and video transcription for Apple Silicon Macs.

**Short description:** LocalScribe turns AAC, WAV, MP3, M4A, MP4, MOV, and WebM files
into editable transcripts with local Whisper models. Media stays on the Mac;
network access is used only for model downloads requested by the user.

**Audience:** journalists, researchers, interviewers, developers, and Mac users
who handle recordings they do not want to upload to a transcription service.

**Proof points:**

- Whisper/whisper.cpp inference runs locally with Metal or CPU.
- No telemetry or cloud speech API.
- Editable timestamps and optional experimental speaker labels.
- TXT and Markdown export.
- MIT-licensed source and a reproducible LGPL-only FFmpeg sidecar.

## Launch order

1. Publish the signed/notarized draft release only after the clean-Mac test.
2. Upload `site/assets/social-preview.png` in GitHub repository settings.
3. Enable GitHub Pages from the Actions workflow and set the repository homepage.
4. Publish the release and post the short English/Russian announcements below.
5. Share in relevant macOS communities only after reading their current rules.
6. Submit a Show HN only when the DMG is immediately runnable and the maintainer
   can stay in the thread to answer technical questions.
7. Use Product Hunt after the first user feedback has removed obvious onboarding
   problems; treat it as a separate launch moment.

## English announcement

I built LocalScribe, an open-source macOS app for private audio and video
transcription. It runs Whisper locally on Apple Silicon, keeps media on the
device, and exports editable transcripts with timestamps.

No cloud upload. No telemetry. AAC, WAV, MP3, M4A, MP4, MOV, and WebM.

Try it and tell me where the workflow still feels rough:
https://github.com/ukolov-dev/LocalScribe

## Russian announcement

Собрал LocalScribe — open-source приложение для локальной расшифровки аудио и
видео на Apple Silicon Mac. Whisper работает прямо на устройстве, записи не
уходят в облако, результат можно редактировать и экспортировать с таймкодами.

Без телеметрии. Поддерживаются AAC, WAV, MP3, M4A, MP4, MOV и WebM.

Буду рад обратной связи по реальному рабочему процессу:
https://github.com/ukolov-dev/LocalScribe

## r/MacApps draft

**Title:** `[OS] LocalScribe — offline Whisper transcription for Apple Silicon (free, open source)`

**Body:**

I made LocalScribe because I wanted a small Mac transcription workflow where
the recording never has to leave the device. It uses local Whisper models,
accepts common audio/video formats, keeps timestamps editable, and exports TXT
or Markdown.

The current release targets Apple Silicon and macOS 13+. Speaker labels are
experimental, and I am especially looking for feedback on model setup,
long-recording behavior, and transcript editing.

Source and signed DMG: https://github.com/ukolov-dev/LocalScribe

Before posting, check the current r/MacApps rules. As of the 2026 rule updates,
open-source posts use the `[OS]` prefix, developer promotion is limited, and a
minimum amount of subreddit participation may be required:
https://www.reddit.com/r/macapps/comments/1qghsc5/new_post_guidelines_and_updates_on_rmacapps/

## Hacker News outline

Suggested factual title:

`Show HN: LocalScribe – offline Whisper transcription for Apple Silicon Macs`

Link directly to the repository/release rather than only to the landing page.
The maintainer should personally write the submission comment around:

- why private recordings motivated the project;
- the Tauri + Rust + whisper.cpp architecture;
- the LGPL-only FFmpeg build and reproducible source attachment;
- what is solid today and what is still experimental;
- the specific feedback being requested.

Hacker News explicitly asks for something people can run without barriers and
prohibits generated or AI-edited discussion text. Do not paste generated launch
copy into the thread; write it personally and remain available to answer:
https://news.ycombinator.com/showhn.html

## Product Hunt fields

- **Name:** LocalScribe
- **Tagline:** Private offline transcription for Apple Silicon Macs
- **Description:** Turn audio and video into editable transcripts with local
  Whisper models. No cloud upload, no telemetry, and open-source code.
- **Primary link:** https://ukolov-dev.github.io/LocalScribe/
- **Maker comment theme:** why recordings should not need to leave the Mac,
  how the local pipeline works, what the alpha supports, and which feedback is
  most useful.

Product Hunt permits makers to post their own product and says not to ask
people directly for upvotes; ask for visits and feedback instead:
https://www.producthunt.com/launch

## Assets

- GitHub/Open Graph: `site/assets/social-preview.png` — 1280×640, 57 KB.
- Deterministic card source: `site/social-card.html`.
- App screenshot: `doc/screenshots/localscribe-transcript.png`.
- Model manager screenshot: `doc/screenshots/localscribe-models.png`.

GitHub recommends a solid-background PNG/JPG/GIF under 1 MB and 1280×640 for
best display:
https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview

## Measurement without product telemetry

Review once per week for the first month:

- GitHub stars and unique repository visitors;
- release DMG downloads;
- landing-page traffic from GitHub Pages insights where available;
- number of reproducible bug reports;
- number of first-time contributors;
- which announcement produced useful discussion, not only clicks.

Keep telemetry out of the app. Repository and release-level aggregate signals
are enough for the first launch.
