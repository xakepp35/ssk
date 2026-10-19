# Linux 192.168.50.155 ↔ Windows 11: быстрый старт

## Сыграть вдвоём

1. Оба компьютера должны быть в одной обычной LAN/Wi-Fi без client isolation.
2. На Linux из корня репозитория:

   ```sh
   ./target/release/ssk
   ```

   Корневой `ssk.yml` задаёт `mode: host`: игра открывает окно 1600×1000 и
   принимает подключения. Отдельный сервер одновременно запускать не нужно.
3. Перенесите **новый** `dist/ssk-windows-gnu.zip` на ноутбук, распакуйте
   в отдельную папку и запустите `ssk.exe`. В архиве рядом лежит `ssk.yml`
   с `mode: join` и `connect: 192.168.50.155:4202`. Аргументы не нужны.
4. Разрешите `ssk.exe` **частную сеть** в Windows Firewall. На Linux должны
   быть доступны из LAN **TCP 4202**, **UDP 4200** и **UDP 4201**.
   TCP нужен только для получения актуального приглашения. Игровые пакеты
   передаются по UDP/QUIC: 360 тиков/с, один пакет каждые 6 тиков, 60 Гц.
5. В HUD появятся peer, RTT и соперник/указатель направления. Полёт: WASD,
   мышь, ЛКМ/Space; оружие 1–3/Q/E; способность Shift/ПКМ.

**F2 / Esc** — игровое меню и выбор корабля. В сети выбранный класс
применяется при следующем респауне (через 2 с после смерти), чтобы смена
не давала мгновенного лечения в бою. Локально кнопка New Run начинает
новый забег выбранным кораблём. Выбор сохраняется. `--class` — только
необязательная стартовая настройка, пользоваться ей не нужно.
**F3** копирует приглашение для ручного подключения, **F11** — fullscreen.
Онлайн-меню не ставит бой на паузу. Частота рендера зависит от vsync монитора.

Каждый компьютер создаёт собственный `runtime/identity.key`; переносите
только файлы из ZIP, не копируйте runtime или чужие ключи.

## Сервер без окна

```sh
./target/release/ssk server
```

Это отдельный P2P-узел rendezvous/discovery без корабля. Он публикует
приглашение на TCP 4202 и знакомит игроков; корабли и общие объекты
считаются по действующей P2P-модели. `node` оставлен как псевдоним.
Для игры на этом же Linux-компьютере используйте обычный **host**, который
совмещает приём подключений и игрока. Не запускайте host и server
одновременно с одинаковыми ключом/портами.

## Конфиг и другие режимы

По умолчанию игра ищет `ssk.yml` сначала в текущей папке, затем рядом с
исполняемым файлом. `--config PATH` остаётся для собственного конфига.
Относительные пути ключей, кэша и сертификатов считаются от папки конфига.
Файл содержит `mode`, `connect`, `window` и секцию `network`; старые плоские
конфиги нужно перенести под `network:`. Папка examples удалена.

- `ssk` — режим из конфига; Linux: host, Windows-архив: join.
- `ssk host` — играть и принимать подключения.
- `ssk online` — подключиться по `connect` из конфига.
- `ssk online --connect IP:4202` либо `--connect 'ssk1:…'` — другой узел.
- `ssk play` — локальный survival.
- `ssk init-config --config other.yml` — создать отдельный конфиг.

При смене IP Linux поправьте `connect` в Windows-конфиге. Если связи нет,
сначала запустите Linux host, проверьте firewall, IP и изоляцию Wi-Fi,
затем перезапустите клиент. Bootstrap по IP рассчитан на доверенную LAN;
сама QUIC-сессия проверяет идентичность и совместимость правил.
Для двух процессов на одной машине создайте два конфига с разными
`network.identity_file`, `network.udp_bind`, `network.discovery_bind`
(клиенту можно порты `0`), host оставьте TCP 4202.

## Сборка

Linux: `cargo build --locked --release -p ssk --jobs 2`.
Linux → Windows: `tools/build-windows.sh` (Rust windows-gnu + MinGW-w64).
Архив содержит **ssk.exe, ssk.yml, PLAY.md**, без внешних runtime DLL.
Скрипт включает статическую линковку compiler runtime и проверяет PE imports;
системные DLL Windows используются штатно и не распространяются с игрой.
[Флаги CRT Rust](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes),
[статическая линковка GCC](https://gcc.gnu.org/onlinedocs/gcc/Link-Options.html).

Для нативной сборки Windows установите Rust MSVC, Visual Studio C++ Build
Tools и Windows SDK ([Microsoft](https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup)).
Из Developer PowerShell в репозитории:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\build-windows.ps1
```

Результат `dist/ssk-windows-msvc.zip`; CRT также задан статически.
На ноутбуке с готовым ZIP Rust/Build Tools не нужны.
Живой матч Linux↔Windows и запуск MSVC-скрипта ещё не проверены.
