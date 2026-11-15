import type { Messages } from "./en";
import { ru as plural } from "./plural";

export const ru: Messages = {
  meta: {
    name: "Русский",
    short: "RU",
  },

  nav: {
    label: "Разделы",
    overview: "Обзор",
    heating: "Нагрев",
    limit: "Лимит",
    history: "История",
    diagnostics: "Диагностика",
    devices: "Устройства",
    console: "Консоль",
    language: "Язык",
  },

  connection: {
    idle: "Не подключено",
    connecting: "Подключение…",
    connected: "Подключено",
    reconnecting: "Переподключение…",
  },

  activity: {
    standby: "Ожидание",
    preheating: "Прогрев",
    heating: "Нагрев",
    charging: "Зарядка",
  },

  modes: {
    delicate: "Деликатный",
    balanced: "Сбалансированный",
    intense: "Интенсивный",
    batterySaver: "Экономия заряда",
  },

  lock: {
    locked: "Заблокировано",
    unlocked: "Разблокировано",
    featureOff: "Блокировка выключена",
  },

  errors: {
    notConnected: "Устройство не подключено.",
    busy: "Устройство занято: идёт запись профиля.",
    deviceNotFound: "Устройство не найдено поблизости.",
    bluetoothUnavailable: "Bluetooth выключен или у Ploom нет к нему доступа.",
    timeout: "Устройство не ответило вовремя.",
    deviceRejected: "Устройство отклонило команду.",
    calibrationOutOfRange:
      "Калибровка устройства вне безопасного диапазона, поэтому профиль не записан.",
    invalidInput: "Неверные данные.",
    dangerousFrame: "Этот кадр нужно подтвердить перед отправкой.",
    cooldownActive: "Идёт перерыв. Это можно сделать, когда он закончится.",
    internal: "Что-то пошло не так.",
  },

  danger: {
    erasesLogs: "Стирает журналы, сохранённые на устройстве. Восстановить их нельзя.",
    disablesBluetooth:
      "Выключает Bluetooth на устройстве. Включить его снова можно только на самом устройстве.",
    entersFirmwareUpdate:
      "Переводит устройство в режим обновления прошивки. До обновления оно не будет работать как обычно.",
  },

  feed: {
    connected: (name: string | null) => (name ? `Подключено к ${name}` : "Подключено"),
    connectionLost: "Связь потеряна",
    disconnected: "Отключено",
    state: {
      standby: "Режим ожидания",
      preheating: "Начался прогрев",
      heating: "Прогрев завершён, идёт нагрев",
      charging: "Зарядка",
    },
    charging: (on: boolean) => (on ? "Зарядка началась" : "Зарядка остановлена"),
    vibration: (on: boolean) => (on ? "Вибрация включена" : "Вибрация выключена"),
    lock: {
      locked: "Устройство заблокировано",
      unlocked: "Устройство разблокировано",
      featureOff: "Функция блокировки выключена",
    },
    autoStart: (on: boolean) => (on ? "Автостарт включён" : "Автостарт выключен"),
    profileApplied: (profile: string, confirmed: boolean) =>
      confirmed
        ? `Профиль применён: ${profile}`
        : `Профиль записан, но устройство не подтвердило: ${profile}`,
    sessionLog: "Журнал сессии прочитан",
    limitReached: (sticks: number, time: string) =>
      `Лимит в ${sticks} ${plural(sticks, "стик", "стика", "стиков")} достигнут, блокировка до ${time}`,
    cooldownEnded: "Перерыв окончен, устройство разблокировано",
    relocked: "Блокировку сняли со стороны, устройство заблокировано снова",
    rawFrame: (replies: number) =>
      replies === 0
        ? "Кадр отправлен, ответа нет"
        : `Кадр отправлен, ${replies} ${plural(replies, "ответ", "ответа", "ответов")}`,
    empty: "Пока пусто. События появятся здесь, когда устройство о них сообщит.",
  },

  format: {
    minSec: (m: number, s: number) => (s === 0 ? `${m} мин` : `${m} мин ${s} с`),
    sec: (s: number) => `${s} с`,
    min: (m: number) => `${m} мин`,
    hourMin: (h: number, m: number) => (m === 0 ? `${h} ч` : `${h} ч ${m} мин`),
    justNow: "Только что",
  },

  common: {
    cancel: "Отменить",
    close: "Закрыть",
    done: "Готово",
    tryAgain: "Повторить",
    on: "Вкл.",
    off: "Выкл.",
    yes: "Да",
    no: "Нет",
    dash: "—",
    notReported: "Пока нет данных",
    reload: "Прочитать снова",
    copy: "Скопировать",
    copied: "Скопировано",
  },

  shell: {
    desktopOnly: "Этот интерфейс работает внутри приложения Ploom.",
    noDevice: "Нет устройства",
    battery: (pct: number) => `Заряд ${pct}%`,
  },

  overview: {
    title: "Обзор",
    firmware: (v: string) => `Прошивка ${v}`,
    hardware: (v: string) => `Аппаратная версия ${v}`,
    curve: {
      label: "Кривая нагрева",
      custom: (temp: number, duration: string) => `Своё удержание: ${temp} °C, ${duration}`,
      ready: "Готов",
      warm: "Остывает",
      warmHint: "Устройство ещё слишком тёплое, чтобы начать новую сессию.",
      preheating: "Прогрев",
      heating: "Идёт сессия",
      elapsed: (elapsed: string, total: string) => `${elapsed} из ${total}`,
      approxElapsed: (elapsed: string, total: string) => `${elapsed} из ~${total}`,
      approxTotal: (total: string) => `~${total}`,
      progressLabel: "Ход сессии",
      balancedTitle: "Заводская кривая",
      balancedBody:
        "Сбалансированный режим использует профиль, встроенный в устройство. Его кривую нельзя прочитать по Bluetooth, поэтому сессия показана относительно её номинальной длительности.",
      unknownMode: "Ждём, пока устройство сообщит режим нагрева.",
      axisMinutes: "мин",
      chartLabel: (mode: string) => `Кривая нагрева «${mode}»: температура во времени`,
    },
    battery: "Аккумулятор",
    charging: "Заряжается",
    plugged: "Подключено к питанию",
    // Short captions inside the battery dial.
    dial: { charge: "Заряд", charging: "Зарядка", plugged: "От сети" },
    health: "Состояние аккумулятора",
    signal: "Сигнал",
    enclosure: "Корпус",
    controls: "Устройство",
    lockTitle: "Блокировка",
    lockHint: "Нагрев не начнётся, пока устройство заблокировано.",
    lockUnavailable: "Функция блокировки выключена на устройстве.",
    lockCooldown: "Заблокировано лимитом до конца перерыва.",
    limitLeft: (left: number, total: number) => `Осталось ${left} из ${total}`,
    limitReached: "Лимит достигнут, блокировка после этой сессии",
    limitLocked: (remaining: string) => `Перерыв: ${remaining}`,
    limitPending: "Перерыв окончен, разблокируется при подключении",
    limitOpen: "Открыть настройки лимита",
    autoStartTitle: "Автостарт",
    autoStartHint: "Начинать нагрев, когда вставлен стик.",
    findTitle: "Найти устройство",
    findHint: "Вибрирует до 30 секунд.",
    findStart: "Включить вибрацию",
    findStop: "Остановить",
    recent: "Недавние события",
    showAll: "Показать все",
    showLess: "Свернуть",
  },

  device3d: {
    label: (finish: string) =>
      `Трёхмерная модель устройства, цвет «${finish}». Потяните, чтобы повернуть.`,
    labelCustom: "Трёхмерная модель устройства. Потяните, чтобы повернуть.",
    unknownColor: (code: string) => `Неизвестный цвет (${code})`,
    finishes: {
      black: "Чёрный",
      silver: "Серебристый",
      gold: "Золотой",
      blue: "Синий",
      glacier: "Ледяной белый",
      teal: "Бирюзовый",
    },
  },

  limit: {
    title: "Лимит",
    enable: "Лимит стиков",
    usedLabel: (used: number, total: number) =>
      `Использовано ${used} из ${total} ${plural(total, "стика", "стиков", "стиков")}`,
    span: (minutes: number) => {
      const h = Math.floor(minutes / 60);
      const m = minutes % 60;
      const hours = h ? `${h} ${plural(h, "час", "часа", "часов")}` : "";
      const mins = m ? `${m} ${plural(m, "минуту", "минуты", "минут")}` : "";
      return [hours, mins].filter(Boolean).join(" ");
    },
    offLead:
      "Ploom может считать стики и блокировать устройство на перерыв, когда вы дойдёте до своего числа.",
    turnOn: "Включить лимит",
    usedCaption: "Выкурено стиков",
    used: (used: number, total: number) => `${used} из ${total}`,
    leftThen: (left: number, span: string) =>
      left === 1
        ? `Остался последний стик. После него перерыв на ${span}.`
        : `${plural(left, "Остался", "Осталось", "Осталось")} ${left} ${plural(left, "стик", "стика", "стиков")}. Потом перерыв на ${span}.`,
    reached: "Лимит достигнут. Устройство заблокируется, когда закончится эта сессия.",
    resets: (time: string) => `Счётчик обнулится в ${time}.`,
    fresh: "Отсчёт начнётся со следующего стика.",
    breakTitle: "Перерыв",
    breakUntil: (time: string) => `До ${time}`,
    remainingLabel: "Время до разблокировки",
    breakNote: "Изменить лимит можно будет после перерыва — сейчас только ужесточить.",
    pendingTitle: "Перерыв окончен",
    pendingBody: "Устройство разблокируется, как только подключится.",
    settings: "Настройки",
    saved: "Сохранено",
    after: "После",
    sticksWord: (n: number) => plural(n, "стика", "стиков", "стиков"),
    lockFor: "блокировать на",
    sticksLabel: "Стиков до перерыва",
    cooldownLabel: "Длительность перерыва",
    other: "Другое…",
    custom: (value: string) => `Своё: ${value}`,
    hoursUnit: "ч",
    minutesUnit: "мин",
    hoursLabel: "Часы",
    minutesLabel: "Минуты",
    decrease: "Уменьшить",
    increase: "Увеличить",
    advanced: "Дополнительно",
    windowLead: "Счётчик обнуляется через",
    windowTail: "после первого стика",
    windowLabel: "Сброс счётчика",
    hours: (h: number) => `${h} ч`,
    turnOff: "Выключить лимит",
    strictOnly:
      "Во время перерыва лимит можно менять только в строгую сторону: меньше стиков, дольше перерыв.",
    offDuringBreak: "Выключить лимит можно после перерыва.",
    how: "Как это работает",
    howBody:
      "Заблокированное устройство не греет, а пока Ploom подключён, он блокирует его снова, если блокировку сняли. Пароля у устройства нет, поэтому без Ploom на связи официальное приложение может его разблокировать: это помощь в самоконтроле, а не сейф. Стики, выкуренные без связи, берутся из счётчиков самого устройства при следующем подключении.",
  },

  gate: {
    title: "Ploom не подключён",
    body: "Подключите его, чтобы продолжить.",
    searching: "Ищем ваш Ploom…",
    connecting: "Подключение…",
    notFoundTitle: "Ploom рядом не найден",
    notFoundBody: "Поиск продолжается в фоне.",
    search: "Найти устройства",
    searchAgain: "Искать снова",
    choose: "Выберите устройство",
    foundNone: "Ничего не найдено. Разбудите устройство и повторите поиск.",
    connectTo: (name: string) => `Подключить ${name}`,
    signalUnknown: "Сигнал неизвестен",
  },

  disconnected: {
    idleTitle: "Не подключено",
    idleBody: "Подключитесь к ближайшему Ploom или выберите устройство из списка.",
    connectingTitle: "Подключение…",
    connectingBody: "Связываемся с устройством. Обычно это занимает несколько секунд.",
    reconnectingTitle: "Ищем ваш Ploom",
    reconnectingBody: "Связь прервалась. Ploom продолжает попытки в фоне.",
    bluetoothTitle: "Bluetooth недоступен",
    bluetoothBody:
      "Включите Bluetooth и разрешите доступ для Ploom в Системных настройках › Конфиденциальность и безопасность › Bluetooth.",
    hintWake: "Нажмите кнопку на устройстве, чтобы его разбудить.",
    hintExclusive:
      "Если открыто официальное приложение или app.ploom.com, закройте их: устройство держит только одно подключение.",
    connect: "Подключить",
    chooseDevice: "Выбрать устройство",
    needsConnection: "Подключите устройство, чтобы пользоваться этим разделом.",
  },

  heating: {
    title: "Нагрев",
    modes: "Режимы",
    modesFooter:
      "Сбалансированный — заводской профиль и надёжный запасной вариант. Остальные режимы записываются в устройство как профиль нагрева из 38 кадров.",
    sticks: (n: number) => `До ${n} ${plural(n, "стика", "стиков", "стиков")} на одном заряде`,
    minutes: (n: number) => `${n} мин`,
    intensity: (n: number) => `Интенсивность ${n} из 3`,
    active: "Активен",
    activeCustom: "Активен, свой",
    apply: "Применить",
    applyBalanced: "Вернуть сбалансированный",
    sheet: {
      title: (mode: string) => `Применить режим «${mode}»?`,
      titleCustom: "Применить свой профиль?",
      body: "Ploom запишет в устройство профиль нагрева из 38 кадров. Это займёт около 8 секунд; держите устройство рядом, пока запись не закончится.",
      fallback: "Вернуться к сбалансированному, заводскому профилю, можно в любой момент.",
      confirm: "Применить профиль",
      progress: (done: number, total: number) => `Записывается кадр ${done} из ${total}`,
      preparing: "Подготовка…",
      success: "Профиль применён",
      successBody: (mode: string) => `Теперь устройство нагревает в режиме «${mode}».`,
      failed: "Профиль не применён",
      busyHeating: "Дождитесь конца сессии, прежде чем менять профиль.",
    },
    custom: {
      title: "Свой профиль",
      footer:
        "Свой профиль меняет только фазу удержания пресета: её температуру и длительность. Всё остальное, включая калибровку безопасности, берётся из базового режима.",
      base: "Основа",
      holdTemp: "Температура удержания",
      holdDuration: "Длительность удержания",
      preview: "Предпросмотр",
      previewLabel: "Предпросмотр своей кривой, фаза удержания выделена",
      showFrames: "Показать кадры",
      apply: "Применить профиль",
      label: (base: string, temp: number, duration: string) =>
        `Свой на основе «${base}», ${temp} °C, ${duration}`,
      unchanged: "Как в пресете",
      total: (duration: string) => `Сессия ${duration}`,
    },
    frames: {
      title: "Кадры профиля",
      subtitle: (count: number) =>
        `${count} ${plural(count, "кадр", "кадра", "кадров")}, отправляются в этом порядке`,
      calibration: (key: number) => `Ключ калибровки ${key}`,
      noCalibration: "Калибровка не нужна",
      awaits: (opcode: string) => `ждёт ${opcode}`,
    },
  },

  history: {
    title: "История",
    read: "Прочитать журналы",
    reading: "Читаем журналы…",
    session: "Сессия: самая ранняя запись",
    sessionEmpty: "На устройстве нет неподтверждённых журналов сессий.",
    duration: "Длительность",
    puffs: "Затяжки",
    puffCount: (n: number) => `${n} ${plural(n, "затяжка", "затяжки", "затяжек")}`,
    timelineLabel: (n: number, duration: string) =>
      `Шкала затяжек: ${n} ${plural(n, "затяжка", "затяжки", "затяжек")} за ${duration}`,
    puffStop: "Остановка по затяжкам",
    autoStart: "Автостарт",
    lock: "Блокировка",
    counter: "Счётчик устройства",
    charge: "Зарядка: самая ранняя запись",
    chargeEmpty: "На устройстве нет неподтверждённых журналов зарядки.",
    voltage: "Напряжение",
    voltageRange: (from: string, to: string) => `с ${from} до ${to} В`,
    chargeCount: "Циклы зарядки",
    soh: "Состояние аккумулятора",
    counterRange: (from: string, to: string) => `с ${from} по ${to}`,
    queue: "Ждут на устройстве",
    queueValue: (sessions: number, charges: number) =>
      `${sessions} ${plural(sessions, "сессия", "сессии", "сессий")}, ${charges} ${plural(charges, "зарядка", "зарядки", "зарядок")}`,
    note: "Устройство отдаёт только самый ранний неподтверждённый журнал. Ploom никогда не удаляет журналы, поэтому здесь остаётся та же запись, пока её не подтвердит другое приложение.",
    notRead: "Журналы ещё не прочитаны.",
  },

  diagnostics: {
    title: "Диагностика",
    reading: "Читаем диагностику…",
    power: "Аккумулятор и питание",
    battery: "Заряд",
    health: "Состояние",
    cycles: "Циклы зарядки",
    usb: "USB-C",
    usbPlugged: "Подключён",
    usbUnplugged: "Не подключён",
    cc: "Линии CC",
    ccHint: "Зарядное устройство поднимает одну из линий выше 100.",
    thermal: "Температура",
    enclosure: "Корпус",
    enclosureHeating: "Во время нагрева не измеряется",
    starts: "Запуски нагрева за всё время",
    startsTotal: (n: string) => `Всего ${n}`,
    startKeys: {
      coldStart: "Холодный старт",
      hotStart: "Горячий старт",
      buttonInAutoWindow: "Кнопкой в окне автостарта",
      buttonAfterTimeout: "Кнопкой после тайм-аута",
      buttonAfterUnavailable: "Кнопкой после недоступности",
      buttonAfterTimeoutOrUnavailable: "Кнопкой: тайм-аут или недоступность",
      autoAfterUnavailable: "Автостарт после недоступности",
    },
    adc: "Сырые показания АЦП",
    adcKeys: {
      Vtemp: "Датчик температуры",
      USBcc1: "USB-C CC1",
      USBcc2: "USB-C CC2",
      HeaterNtc: "Термистор нагревателя",
      B: "Аккумулятор",
      CaseNtc: "Термистор корпуса",
      HeatTemp: "Температура нагревателя",
    },
    adcFooter: "Немасштабированные значения прямо из аналого-цифрового преобразователя устройства.",
    name: "Канал",
    value: "Значение",
  },

  devices: {
    title: "Устройства",
    current: "Текущее устройство",
    none: "Устройство не подключено",
    connectFirst: "Подключить ближайшее",
    disconnect: "Отключить",
    nearby: "Рядом",
    scan: "Искать",
    scanning: "Поиск…",
    connect: "Подключить",
    switch: "Переключиться",
    thisDevice: "Подключено",
    identifier: "Идентификатор",
    hardware: "Аппаратная версия",
    firmware: "Прошивка",
    signal: (dbm: number) => `${dbm} дБм`,
    noSignal: "Уровень сигнала неизвестен",
    notScanned: "Запустите поиск, чтобы найти Ploom поблизости.",
    nothingFound: "Устройства не найдены.",
    warning:
      "В протоколе Ploom нет сопряжения: любым устройством в радиусе действия может управлять кто угодно рядом. Подключайтесь только к своему.",
  },

  console: {
    title: "Консоль",
    intro:
      "Отправьте сырой кадр и посмотрите, что ответит устройство. Кадр — это [LEN][OPCODE][данные] в hex; LEN — число байтов после него.",
    frame: "Кадр",
    placeholder: "01 e3",
    send: "Отправить",
    sending: "Отправка…",
    presets: "Частые кадры",
    preset: {
      status: "Статус",
      rssi: "Сигнал",
      profile: "Флаг профиля",
      mode: "Режим",
      enclosure: "Температура корпуса",
    },
    valid: (opcode: string, payload: number) =>
      `Опкод ${opcode}, ${payload} ${plural(payload, "байт", "байта", "байтов")} данных`,
    invalid: {
      empty: "Введите байты в hex, например 01 e3.",
      chars: "Допустимы только hex-цифры 0–9 и a–f.",
      odd: "Нечётное число цифр: на каждый байт нужно две.",
      short: "В кадре должно быть хотя бы два байта: LEN и OPCODE.",
      len: (expected: string, actual: string) => `LEN должен быть ${expected}, а не ${actual}.`,
    },
    dangerous: "Опасный опкод",
    dangerSheet: {
      title: (opcode: string) => `Отправить ${opcode}?`,
      confirm: "Отправить кадр",
    },
    replies: "Ответы",
    noReplies: "За время ожидания ответа не было.",
    sent: "Отправлено",
    emptyLog: "Здесь появятся отправленные кадры и ответы на них.",
    clear: "Очистить",
  },
};
