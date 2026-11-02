# Sandbox `$ polecenie $` (Zero-Cost Sandbox)

```
$ ls -la /tmp $
```

Polecenie jest uruchamiane przez `sh -c` w izolacji. Wewnątrz bloku nie wolno
używać znaku `$` (jest ogranicznikiem), więc np. `$(...)` i `$?` trzeba zastąpić
inną konstrukcją albo wynieść do skryptu.

## Izolacja (0.6)

Metody są próbowane po kolei, pierwsza, która się uda, wygrywa:

1. `unshare(CLONE_NEWNET | CLONE_NEWNS)` + bind-remount `/` jako tylko-do-odczytu
   (wymaga `CAP_SYS_ADMIN`);
2. to samo przez `CLONE_NEWUSER` (user namespace z mapowaniem uid/gid) — działa bez
   roota, o ile jądro/AppArmor pozwala na nieuprzywilejowane user namespace;
3. `bwrap` (bubblewrap), jeśli jest zainstalowany i działa.

Efekt: proces widzi tylko interfejs `lo` (brak sieci), a system plików jest
tylko do odczytu. Błąd izolacji jest wykrywany jako błąd `spawn`, nie po kodzie
wyjścia polecenia.

Gdy żadna metoda nie działa: domyślnie (`HKS_SANDBOX` nieustawione albo `strict`)
program **panikuje i polecenie nie jest wykonywane** (fail-closed).
`HKS_SANDBOX=best-effort` uruchamia je jawnie bez izolacji (z ostrzeżeniem na
stderr), `HKS_SANDBOX=off` pomija izolację w ogóle.

## seccomp (metody 1 i 2)

Po założeniu izolacji, przed `exec`, proces dostaje `PR_SET_NO_NEW_PRIVS` i filtr
seccomp-BPF. To **denylista**: wywołania systemowe poniżej zwracają `EPERM`.

`ptrace`, `mount`, `umount2`, `pivot_root`, `chroot`, `reboot`, `kexec_load`,
`init_module`, `finit_module`, `delete_module`, `swapon`, `swapoff`, `bpf`,
`perf_event_open`, `setns`, `unshare`, `keyctl`, `add_key`, `request_key`,
`process_vm_readv`, `process_vm_writev`, `userfaultfd`, `open_by_handle_at`,
`acct`, `settimeofday`, `clock_settime`, `syslog`.

Filtr sprawdza też architekturę (niezgodna = zabicie procesu) i blokuje ABI x32
na x86-64 (inaczej numery wywołań z bitem `0x40000000` omijałyby listę).
Obsługiwane architektury: x86-64 i aarch64 (na innych filtr nie jest zakładany).

Wyłączenie: `HKS_SANDBOX_SECCOMP=off`. Sprawdzenie z poziomu polecenia:
`grep Seccomp: /proc/self/status` daje `2`, gdy filtr jest założony.

**Ograniczenia.** To denylista, nie allowlista — zmniejsza powierzchnię ucieczki,
ale nie jest pełnym modelem uprawnień (np. nie ogranicza `open`/`connect` w obrębie
dozwolonych zasobów). Metoda 3 (`bwrap`) nie dostaje tego filtra, bo zablokowałby
własne wywołania `bwrap`.

## Limity zasobów (wszystkie metody, domyślnie bez limitu)

| Zmienna | Limit | Jednostka |
|---|---|---|
| `HKS_SANDBOX_CPU` | `RLIMIT_CPU` | sekundy CPU |
| `HKS_SANDBOX_MEM_MB` | `RLIMIT_AS` | MB przestrzeni adresowej |
| `HKS_SANDBOX_NPROC` | `RLIMIT_NPROC` | liczba procesów |
| `HKS_SANDBOX_FSIZE_MB` | `RLIMIT_FSIZE` | MB pojedynczego pliku |
| `HKS_SANDBOX_NOFILE` | `RLIMIT_NOFILE` | liczba deskryptorów |

Limity są ustawiane w procesie potomnym przez `setrlimit` (miękki = twardy).
`RLIMIT_CPU` liczy czas CPU, nie zegarowy.

## Testy

`tests/sandbox.sh` (uruchamiany przez `scripts/verify.sh`) sprawdza założenie filtra
(`Seccomp: 2`), wyłączenie (`Seccomp: 0`), blokadę `unshare` i limity. Gdy
środowisko nie daje izolacji, test się pomija (komunikat `skip`).

## Znane ograniczenia

* Remount tylko-do-odczytu obejmuje montowanie `/`; osobne montowania (np. osobny
  `/home`) mogą pozostać zapisywalne.
* Brak izolacji PID/IPC; brak Windows/macOS.
* Nie sprawdzono metody 3 (`bwrap`) ani maszyny z wyłączonymi user namespace na
  poziomie jądra.
