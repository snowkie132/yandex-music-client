# mashiro (v2) — локальная сборка и правки под Linux

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
   на loopback; фронтенд (`src/lib/stream.ts`, `src/boot/stream.ts`) берёт его адрес
   командой `stream_base`.
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