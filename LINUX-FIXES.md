# mashiro (v2) — локальная сборка и правки

## Где что лежит

- исходники: `~/mashiro` (ветка `v2` + локальные коммиты)
- установленный бинарник: `~/.local/bin/mashiro`
- ярлык: `~/.local/share/applications/mashiro.desktop` (`Exec=/home/snowkie/.local/bin/mashiro`)
- иконка: `~/.local/share/icons/hicolor/512x512/apps/mashiro.png`
- данные/настройки приложения: `~/.local/share/app.mashiro.desktop`
- лог текущего запуска: `~/.local/state/mashiro/mashiro.log`

В `/tmp` ничего нужного для работы нет: бинарник — копия, все ассеты вшиты в него
при сборке, поток идёт через локальный сервер на `127.0.0.1` (порт выбирается
случайно при старте). После перезагрузки достаточно запустить **mashiro** из меню.

## Локальные правки (сверх upstream)

1. `fix(linux): local http stream proxy` — WebKitGTK не умеет проигрывать медиа
   с кастомных схем, поэтому в `src-tauri/src/proxy.rs` поднят обычный HTTP-сервер
   на loopback. Запросы к CDN проксируются как есть (Range и заголовки без изменений),
   переписываются только m3u8-плейлисты. Фронтенд (`src/lib/stream.ts`,
   `src/boot/stream.ts`) берёт адрес сервера командой `stream_base`.
2. `fix(linux): offline iconify collections` — `@iconify-json/{mingcute,simple-icons,gravity-ui,maki}`
   регистрируются через `addCollection` в `src/boot/icons.ts` (без сети).
3. `fix: громкость через WebAudio gain` — `audio.volume` игнорируется, когда звук
   идёт через `AudioContext`; громкость/фейды/mute теперь через `GainNode`
   (`src/lib/audio.ts`, `setOutputVolume`).
4. `lyrics` — `v-memo` для строк, заливка каждый кадр, экстраполяция позиции
   между обновлениями `audio.currentTime`. Караоке — каскад глифов (240 ms,
   стаггер ≤ 16 ms, подъём 3.4 px), яркость по плейхеду, активное слово ×1.13,
   каскадный уход строки. Прокрутка 360 ms cubic-bezier(0.42, 0, 0.58, 1),
   при открытии панель сразу встаёт на текущую строку. Блюр текста убран,
   у фона-обложки остался лёгкий `blur(14px)`.
5. `fix(player)` — при мёртвом потоке (протухший URL/прокси не отдал чанк) трек
   перезагружается с той же позиции со свежим `stream`-URL; сторож раз в 2 с
   возвращает `AudioContext` из `suspended`.
6. `@tauri-apps/api` запинен на `2.11.1` (иначе `tauri build` ругается на
   несовпадение версий с crate `tauri 2.11.5`).

## Пересборка / обновление

```bash
cd ~/mashiro
git pull                                  # если обновляемся с upstream
cd desktop
npm install
npm run tauri:build -- --no-bundle
install -m755 src-tauri/target/release/mashiro ~/.local/bin/mashiro
```

Проверка после установки:

```bash
pgrep -x mashiro || (setsid mashiro > ~/.local/state/mashiro/mashiro.log 2>&1 &)
```

## Окружение

- WebKitGTK на этой машине рисует программно (`MESA-EGL: failed to create dri2 screen`
  при NVIDIA 10de:2582) — тяжёлые блюры/фильтры бьют по fps, поэтому они и убраны.
- В микшере PipeWire видно два потока `mashiro` (`webaudio` и `music`) — это норма:
  WebKit делает отдельный поток на `AudioContext` и на медиаэлемент.

## Android

APK собирается из того же дерева (`src-tauri/gen/android` уже сгенерирован и лежит в репозитории).

```bash
cd ~/mashiro/desktop
export JAVA_HOME=$HOME/android-dev/jdk-17.0.21+9
export ANDROID_HOME=$HOME/android-dev/sdk
export NDK_HOME=$HOME/android-dev/sdk/ndk/27.2.12479018
export PATH=$JAVA_HOME/bin:$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH
npx tauri android build --apk --target aarch64
```

Готовый APK подписывается автоматически (`builds/android/mashiro.keystore`),
результат кладётся в `builds/android/mashiro-arm64.apk`. Для эмулятора собирается
с `--target x86_64`.

Что учтено под телефон:
- боковая панель — оверлей, по умолчанию свёрнута, открывается кнопкой в шапке;
- настройки на узком экране сворачиваются в иконный рейл;
- скрыты оконные кнопки, мини-плеер и трей (в Rust — заглушки `*_mobile.rs`);
- `gen/android/app/src/main/res/xml/network_security_config.xml` разрешает cleartext
  только для `127.0.0.1` — без него WebView не пускает поток с локального прокси;
- безопасные отступы (`env(safe-area-inset-*)`) под вырез и системные панели;
- hotkeys/keyring на Android отключены (сид-фраза «Вместе» хранится в файле).
