# Название: **LocalScribe**

Смысл: локальная расшифровка аудио без облака, с упором на приватность.

**ТЗ: LocalScribe**

**1. Цель**  
Создать macOS desktop-приложение для локальной транскрибации аудио/видео в текст с использованием `whisper-large-v3-turbo`, без Python, без отправки файлов в облако.

**2. Целевая платформа**  
Первый релиз: macOS Apple Silicon, M1/M2/M3/M4.  
Минимально: macOS 13+, 8 ГБ RAM.  
Рекомендуемо: macOS 14+, 16 ГБ+ RAM.  
Оптимальная конфигурация: MacBook Pro M3 Pro 18 ГБ.

**3. Технологии**  
Frontend: Tauri 2 + React + TypeScript + Vite.  
Backend: Rust.  
ASR: `whisper-rs` + `whisper.cpp` + Metal.  
Конвертация аудио: встроенный/sidecar `ffmpeg`.  
Скачивание моделей: `reqwest`, `tokio`, `sha2`.  
Хранение настроек: Tauri app config/store.  
Форматы экспорта: `TXT`, `SRT`, `VTT`, позже `JSON`.

**4. Основной сценарий**  
Пользователь открывает приложение, загружает модель через встроенный загрузчик, выбирает аудио или видеофайл, задаёт язык и опциональный словарь терминов, нажимает `Transcribe`, получает текст и сохраняет результат.

**5. Функции MVP**  
- Drag-and-drop аудио/видео файла в окно приложения.
- Выбор файла через системный диалог.
- Встроенный менеджер моделей.
- Скачивание `large-v3-turbo-q5_0` как модели по умолчанию.
- Возможность выбрать полную `large-v3-turbo`.
- Возможность указать custom `.bin` модель вручную.
- Проверка наличия модели при запуске.
- Прогресс загрузки модели.
- Прогресс обработки аудио.
- Выбор языка: `auto`, `ru`, `en`, список популярных языков.
- Поле `Vocabulary / prompt` для имён, терминов и названий.
- Результат транскрибации в редактируемом текстовом поле.
- Экспорт в `.txt`, `.srt`, `.vtt`.
- Локальная история последних задач без копирования исходных файлов.

**6. Модели**  
По умолчанию предлагать:

```text
large-v3-turbo-q5_0   ~547 MB   Recommended
large-v3-turbo        ~1.5 GB   Best quality
Custom model file
```

Модели хранить в:

```text
~/Library/Application Support/LocalScribe/models/
```

Загрузка должна идти во временный файл:

```text
model.bin.part -> model.bin
```

При сбое загрузки приложение должно показывать ошибку и позволять повторить скачивание.

**7. Поддерживаемые входные форматы**  
Минимум: `mp3`, `wav`, `m4a`, `mp4`, `mov`, `webm`.  
Все входные файлы перед распознаванием приводятся к:

```text
16 kHz, mono, PCM WAV
```

Временные файлы хранить в cache-директории приложения и удалять после завершения задачи.

**8. Настройки распознавания**  
- Язык: auto/manual.
- Initial prompt / vocabulary.
- Timestamps: on/off.
- Output format: TXT/SRT/VTT.
- Модель по умолчанию.
- Количество потоков: auto/manual.
- Квантованная модель по умолчанию: on.
- Автоудаление временных файлов: on.

**9. Интерфейс**  
Главный экран без лендинга:

```text
[Drop audio/video here]

Model: large-v3-turbo-q5_0   [Manage]
Language: Auto / Russian / English
Vocabulary: [optional text field]

[Transcribe]

Progress bar

Transcript editor

[Save TXT] [Save SRT] [Save VTT]
```

Отдельный экран `Models`:

```text
Model name        Size      Status        Action
Turbo Q5          547 MB    Installed     Use / Delete
Turbo Full        1.5 GB    Not installed Download
Custom            -         -             Choose file
```

**10. Архитектура**  
Frontend вызывает Tauri commands:

```rust
list_models()
download_model(model_id)
delete_model(model_id)
select_custom_model()
transcribe_file(options)
cancel_transcription(job_id)
export_transcript(format)
```

Backend отвечает событиями:

```text
model-download-progress
model-download-completed
transcription-progress
transcription-segment
transcription-completed
transcription-error
```

**11. Поведение при ошибках**  
- Нет модели: предложить скачать.
- Нет `ffmpeg`: показать понятную ошибку или использовать bundled binary.
- Неподдерживаемый файл: показать список поддерживаемых форматов.
- Недостаточно памяти: предложить `q5_0`.
- Прерывание задачи: корректно остановить обработку и удалить временные файлы.
- Повреждённая модель: удалить/перекачать.

**12. Критерии готовности MVP**  
- Приложение запускается на macOS Apple Silicon.
- Модель скачивается из UI с прогрессом.
- Файл `mp3` транскрибируется без Python.
- Русская речь распознаётся через `large-v3-turbo-q5_0`.
- Результат можно сохранить в `TXT`, `SRT`, `VTT`.
- После перезапуска приложение помнит выбранную модель.
- Все операции происходят локально.
- UI не зависает во время скачивания и транскрибации.

**13. Следующий этап после MVP**  
- Пакетная обработка файлов.
- Разделение по спикерам, если подключить отдельный diarization pipeline.
- Автоматическое резюме через локальную или облачную LLM.
- Очистка текста: пунктуация, абзацы, удаление повторов.
- Горячая папка: приложение само транскрибирует новые файлы.
- Экспорт в Markdown и DOCX.
