import { en as plural } from "./plural";

/** English dictionary. */
export const en = {
  meta: {
    name: "English",
    short: "EN",
  },

  nav: {
    label: "Sections",
    overview: "Overview",
    heating: "Heating",
    limit: "Limit",
    history: "History",
    diagnostics: "Diagnostics",
    devices: "Devices",
    console: "Console",
    language: "Language",
  },

  connection: {
    idle: "Not connected",
    connecting: "Connecting…",
    connected: "Connected",
    reconnecting: "Reconnecting…",
  },

  activity: {
    standby: "Standby",
    preheating: "Preheating",
    heating: "Heating",
    charging: "Charging",
  },

  modes: {
    delicate: "Delicate",
    balanced: "Balanced",
    intense: "Intense",
    batterySaver: "Battery Saver",
  },

  lock: {
    locked: "Locked",
    unlocked: "Unlocked",
    featureOff: "Lock feature off",
  },

  errors: {
    notConnected: "No device is connected.",
    busy: "The device is busy writing a profile.",
    deviceNotFound: "The device wasn’t found nearby.",
    bluetoothUnavailable: "Bluetooth is off, or Ploom isn’t allowed to use it.",
    timeout: "The device didn’t respond in time.",
    deviceRejected: "The device refused the command.",
    calibrationOutOfRange:
      "This device’s calibration is outside the safe range, so the profile wasn’t written.",
    invalidInput: "The input isn’t valid.",
    dangerousFrame: "This frame needs confirmation before it’s sent.",
    cooldownActive: "A cooldown is running. This has to wait until it ends.",
    internal: "Something went wrong.",
  },

  danger: {
    erasesLogs: "Erases the logs stored on the device. They can’t be recovered.",
    disablesBluetooth:
      "Turns off Bluetooth on the device. It can only be turned back on from the device itself.",
    entersFirmwareUpdate:
      "Puts the device into firmware update mode. It stops working normally until updated.",
  },

  feed: {
    connected: (name: string | null): string => (name ? `Connected to ${name}` : "Connected"),
    connectionLost: "Connection lost",
    disconnected: "Disconnected",
    state: {
      standby: "Back to standby",
      preheating: "Preheating started",
      heating: "Preheat finished, heating",
      charging: "Charging",
    },
    charging: (on: boolean): string => (on ? "Charging started" : "Charging stopped"),
    vibration: (on: boolean): string => (on ? "Vibration started" : "Vibration stopped"),
    lock: {
      locked: "Device locked",
      unlocked: "Device unlocked",
      featureOff: "Lock feature turned off",
    },
    autoStart: (on: boolean): string => (on ? "Auto-start turned on" : "Auto-start turned off"),
    profileApplied: (profile: string, confirmed: boolean): string =>
      confirmed ? `${profile} applied` : `${profile} written, not confirmed by the device`,
    sessionLog: "Session log read",
    limitReached: (sticks: number, time: string): string =>
      `Limit of ${sticks} ${plural(sticks, "stick", "sticks")} reached, locked until ${time}`,
    cooldownEnded: "Cooldown over, device unlocked",
    relocked: "Unlocked elsewhere during the cooldown, locked again",
    rawFrame: (replies: number): string =>
      replies === 0
        ? "Frame sent, no reply"
        : `Frame sent, ${replies} ${plural(replies, "reply", "replies")}`,
    empty: "Nothing yet. Events appear here as the device reports them.",
  },

  format: {
    minSec: (m: number, s: number): string => (s === 0 ? `${m} min` : `${m} min ${s} s`),
    sec: (s: number): string => `${s} s`,
    min: (m: number): string => `${m} min`,
    hourMin: (h: number, m: number): string => (m === 0 ? `${h} h` : `${h} h ${m} min`),
    justNow: "Just now",
  },

  common: {
    cancel: "Cancel",
    close: "Close",
    done: "Done",
    tryAgain: "Try again",
    on: "On",
    off: "Off",
    yes: "Yes",
    no: "No",
    dash: "—",
    notReported: "Not reported yet",
    reload: "Read again",
    copy: "Copy",
    copied: "Copied",
  },

  shell: {
    desktopOnly: "This interface runs inside the Ploom desktop app.",
    noDevice: "No device",
    battery: (pct: number): string => `${pct}% battery`,
  },

  overview: {
    title: "Overview",
    firmware: (v: string): string => `Firmware ${v}`,
    hardware: (v: string): string => `Hardware ${v}`,
    curve: {
      label: "Heating curve",
      custom: (temp: number, duration: string): string => `Custom hold ${temp} °C for ${duration}`,
      ready: "Ready",
      warm: "Cooling down",
      warmHint: "The device is too warm to start another session yet.",
      preheating: "Preheating",
      heating: "In session",
      elapsed: (elapsed: string, total: string): string => `${elapsed} of ${total}`,
      approxElapsed: (elapsed: string, total: string): string => `${elapsed} of ~${total}`,
      approxTotal: (total: string): string => `~${total}`,
      progressLabel: "Session progress",
      balancedTitle: "Factory curve",
      balancedBody:
        "Balanced runs the profile built into the device. Its curve isn’t readable over Bluetooth, so the session is shown against its nominal length.",
      unknownMode: "Waiting for the device to report its heating mode.",
      axisMinutes: "min",
      chartLabel: (mode: string): string => `${mode} heating curve, temperature over time`,
    },
    battery: "Battery",
    charging: "Charging",
    plugged: "Plugged in",
    // Short captions inside the battery dial.
    dial: { charge: "Charge", charging: "Charging", plugged: "Plugged in" },
    health: "Battery health",
    signal: "Signal",
    enclosure: "Enclosure",
    controls: "Device",
    lockTitle: "Lock",
    lockHint: "Heating can’t start until the device is unlocked.",
    lockUnavailable: "The lock feature is turned off on the device.",
    autoStartTitle: "Auto-start",
    autoStartHint: "Start heating when a stick is inserted.",
    findTitle: "Find device",
    findHint: "Vibrates for up to 30 seconds.",
    findStart: "Vibrate",
    findStop: "Stop",
    lockCooldown: "Locked by the stick limit until the cooldown ends.",
    limitLeft: (left: number, total: number): string => `${left} of ${total} left`,
    limitReached: "Limit reached, locks after this session",
    limitLocked: (remaining: string): string => `Break: ${remaining}`,
    limitPending: "Cooldown over, unlocks when the device connects",
    limitOpen: "Open limit settings",
    recent: "Recent activity",
    showAll: "Show all",
    showLess: "Show less",
  },

  device3d: {
    label: (finish: string): string => `3D model of the device in ${finish}. Drag to rotate.`,
    labelCustom: "3D model of the device. Drag to rotate.",
    unknownColor: (code: string): string => `Unknown colour (${code})`,
    finishes: {
      black: "Black",
      silver: "Silver",
      gold: "Gold",
      blue: "Blue",
      glacier: "Glacier White",
      teal: "Teal Electric Pop",
    },
  },

  limit: {
    title: "Limit",
    enable: "Stick limit",
    usedLabel: (used: number, total: number): string =>
      `${used} of ${total} ${plural(total, "stick", "sticks")} used`,
    span: (minutes: number): string => {
      const h = Math.floor(minutes / 60);
      const m = minutes % 60;
      const hours = h ? `${h} ${plural(h, "hour", "hours")}` : "";
      const mins = m ? `${m} ${plural(m, "minute", "minutes")}` : "";
      return [hours, mins].filter(Boolean).join(" ");
    },
    offLead:
      "Ploom can count your sticks and lock the device for a break once you reach your number.",
    turnOn: "Turn on limit",
    usedCaption: "Sticks used",
    used: (used: number, total: number): string => `${used} of ${total}`,
    leftThen: (left: number, span: string): string =>
      left === 1
        ? `One stick left. After it, a break of ${span}.`
        : `${left} sticks left. Then a break of ${span}.`,
    reached: "Limit reached. The device locks when this session ends.",
    resets: (time: string): string => `The counter resets at ${time}.`,
    fresh: "The count starts with the next stick.",
    breakTitle: "Break",
    breakUntil: (time: string): string => `Until ${time}`,
    remainingLabel: "Time until the device unlocks",
    breakNote: "You can change the limit after the break. For now it can only be made stricter.",
    pendingTitle: "Break over",
    pendingBody: "The device unlocks as soon as it connects.",
    settings: "Settings",
    saved: "Saved",
    after: "After",
    sticksWord: (n: number): string => plural(n, "stick", "sticks"),
    lockFor: "lock for",
    sticksLabel: "Sticks before the break",
    cooldownLabel: "Break length",
    other: "Other…",
    custom: (value: string): string => `Custom: ${value}`,
    hoursUnit: "h",
    minutesUnit: "min",
    hoursLabel: "Hours",
    minutesLabel: "Minutes",
    decrease: "Decrease",
    increase: "Increase",
    advanced: "More options",
    windowLead: "The counter resets",
    windowTail: "after the first stick",
    windowLabel: "Counter reset",
    hours: (h: number): string => `${h} h`,
    turnOff: "Turn off limit",
    strictOnly: "During a break the limit can only get stricter: fewer sticks, a longer break.",
    offDuringBreak: "You can turn the limit off after the break.",
    how: "How it works",
    howBody:
      "A locked device won’t heat, and while Ploom is connected it locks the device again if someone unlocks it. The device has no password, so the official app can still unlock it when Ploom isn’t connected: this helps you keep a promise, it isn’t a safe. Sticks used while Ploom was away are read from the device’s own counters on reconnect.",
  },

  gate: {
    title: "Ploom isn’t connected",
    body: "Connect it to continue.",
    searching: "Looking for your Ploom…",
    connecting: "Connecting…",
    notFoundTitle: "No Ploom found nearby",
    notFoundBody: "Still trying in the background.",
    search: "Search for devices",
    searchAgain: "Search again",
    choose: "Choose a device",
    foundNone: "Nothing found. Wake the device and search again.",
    connectTo: (name: string): string => `Connect to ${name}`,
    signalUnknown: "Signal unknown",
  },

  disconnected: {
    idleTitle: "Not connected",
    idleBody: "Connect to the Ploom nearby, or pick one from the list of devices.",
    connectingTitle: "Connecting…",
    connectingBody: "Talking to the device. This usually takes a few seconds.",
    reconnectingTitle: "Looking for your Ploom",
    reconnectingBody: "The connection dropped. Ploom keeps trying in the background.",
    bluetoothTitle: "Bluetooth is unavailable",
    bluetoothBody:
      "Turn on Bluetooth and allow Ploom in System Settings › Privacy & Security › Bluetooth.",
    hintWake: "Press the button on the device to wake it.",
    hintExclusive:
      "If the official app or app.ploom.com is open, close it — the device accepts only one connection at a time.",
    connect: "Connect",
    chooseDevice: "Choose device",
    needsConnection: "Connect a device to use this section.",
  },

  heating: {
    title: "Heating",
    modes: "Modes",
    modesFooter:
      "Balanced is the factory profile and always a safe fallback. The other modes are written into the device as a 38-frame heating profile.",
    sticks: (n: number): string => `Up to ${n} ${plural(n, "stick", "sticks")} per charge`,
    minutes: (n: number): string => `${n} min`,
    intensity: (n: number): string => `Intensity ${n} of 3`,
    active: "Active",
    activeCustom: "Active, custom",
    apply: "Apply",
    applyBalanced: "Switch to Balanced",
    sheet: {
      title: (mode: string): string => `Apply ${mode}?`,
      titleCustom: "Apply custom profile?",
      body: "Ploom will write a heating profile of 38 frames into the device. It takes about 8 seconds; keep the device close until it finishes.",
      fallback: "You can always return to Balanced, the factory profile.",
      confirm: "Apply profile",
      progress: (done: number, total: number): string => `Writing frame ${done} of ${total}`,
      preparing: "Preparing…",
      success: "Profile applied",
      successBody: (mode: string): string => `The device now heats with ${mode}.`,
      failed: "The profile wasn’t applied",
      busyHeating: "Wait until the session ends before changing the profile.",
    },
    custom: {
      title: "Custom profile",
      footer:
        "A custom profile changes only the hold phase of a preset: its temperature and length. Everything else, including the safety calibration, stays from the base mode.",
      base: "Based on",
      holdTemp: "Hold temperature",
      holdDuration: "Hold duration",
      preview: "Preview",
      previewLabel: "Custom curve preview, hold phase highlighted",
      showFrames: "Show frames",
      apply: "Apply profile",
      label: (base: string, temp: number, duration: string): string =>
        `Custom ${base}, ${temp} °C, ${duration}`,
      unchanged: "Same as the preset",
      total: (duration: string): string => `Session ${duration}`,
    },
    frames: {
      title: "Profile frames",
      subtitle: (count: number): string =>
        `${count} ${plural(count, "frame", "frames")}, written in this order`,
      calibration: (key: number): string => `Calibration key ${key}`,
      noCalibration: "No calibration needed",
      awaits: (opcode: string): string => `waits for ${opcode}`,
    },
  },

  history: {
    title: "History",
    read: "Read logs",
    reading: "Reading logs…",
    session: "Oldest session log",
    sessionEmpty: "The device has no unacknowledged session logs.",
    duration: "Duration",
    puffs: "Puffs",
    puffCount: (n: number): string => `${n} ${plural(n, "puff", "puffs")}`,
    timelineLabel: (n: number, duration: string): string =>
      `Puff timeline: ${n} ${plural(n, "puff", "puffs")} over ${duration}`,
    puffStop: "Puff stop",
    autoStart: "Auto-start",
    lock: "Lock",
    counter: "Device counter",
    charge: "Oldest charge log",
    chargeEmpty: "The device has no unacknowledged charge logs.",
    voltage: "Voltage",
    voltageRange: (from: string, to: string): string => `${from} V to ${to} V`,
    chargeCount: "Charge cycles",
    soh: "Battery health",
    counterRange: (from: string, to: string): string => `${from} to ${to}`,
    queue: "Waiting on the device",
    queueValue: (sessions: number, charges: number): string =>
      `${sessions} ${plural(sessions, "session", "sessions")}, ${charges} ${plural(charges, "charge", "charges")}`,
    note: "The device only hands out its oldest unacknowledged log. Ploom never deletes logs, so the same entry stays here until another app acknowledges it.",
    notRead: "Logs haven’t been read yet.",
  },

  diagnostics: {
    title: "Diagnostics",
    reading: "Reading diagnostics…",
    power: "Battery and power",
    battery: "Charge",
    health: "Health",
    cycles: "Charge cycles",
    usb: "USB-C",
    usbPlugged: "Plugged in",
    usbUnplugged: "Not plugged in",
    cc: "CC lines",
    ccHint: "A charger pulls one line above 100.",
    thermal: "Temperature",
    enclosure: "Enclosure",
    enclosureHeating: "Not measured while heating",
    starts: "Heating starts, lifetime",
    startsTotal: (n: string): string => `${n} in total`,
    startKeys: {
      coldStart: "Cold start",
      hotStart: "Hot start",
      buttonInAutoWindow: "Button, during auto-start window",
      buttonAfterTimeout: "Button, after timeout",
      buttonAfterUnavailable: "Button, after unavailable",
      buttonAfterTimeoutOrUnavailable: "Button, after timeout or unavailable",
      autoAfterUnavailable: "Auto-start, after unavailable",
    },
    adc: "Raw ADC readings",
    adcKeys: {
      Vtemp: "Temperature sensor",
      USBcc1: "USB-C CC1",
      USBcc2: "USB-C CC2",
      HeaterNtc: "Heater thermistor",
      B: "Battery",
      CaseNtc: "Case thermistor",
      HeatTemp: "Heater temperature",
    },
    adcFooter: "Unscaled values straight from the device’s analog-to-digital converter.",
    name: "Channel",
    value: "Value",
  },

  devices: {
    title: "Devices",
    current: "Current device",
    none: "No device connected",
    connectFirst: "Connect to nearest",
    disconnect: "Disconnect",
    nearby: "Nearby",
    scan: "Scan",
    scanning: "Scanning…",
    connect: "Connect",
    switch: "Switch",
    thisDevice: "Connected",
    identifier: "Identifier",
    hardware: "Hardware",
    firmware: "Firmware",
    signal: (dbm: number): string => `${dbm} dBm`,
    noSignal: "No signal reading",
    notScanned: "Scan to find Ploom devices nearby.",
    nothingFound: "No devices found.",
    warning:
      "The Ploom protocol has no pairing: any device in range can be controlled by anyone nearby. Only connect to a device that is yours.",
  },

  console: {
    title: "Console",
    intro:
      "Send a raw frame and see what the device answers. A frame is [LEN][OPCODE][payload] in hex; LEN counts the bytes after itself.",
    frame: "Frame",
    placeholder: "01 e3",
    send: "Send",
    sending: "Sending…",
    presets: "Common frames",
    preset: {
      status: "Status",
      rssi: "Signal",
      profile: "Profile flag",
      mode: "Mode",
      enclosure: "Enclosure temperature",
    },
    valid: (opcode: string, payload: number): string =>
      `Opcode ${opcode}, ${payload} ${plural(payload, "payload byte", "payload bytes")}`,
    invalid: {
      empty: "Enter bytes in hex, for example 01 e3.",
      chars: "Use only hex digits 0–9 and a–f.",
      odd: "An odd number of digits; every byte needs two.",
      short: "A frame needs at least two bytes: LEN and OPCODE.",
      len: (expected: string, actual: string): string =>
        `LEN should be ${expected}, not ${actual}.`,
    },
    dangerous: "This opcode is dangerous",
    dangerSheet: {
      title: (opcode: string): string => `Send ${opcode}?`,
      confirm: "Send frame",
    },
    replies: "Replies",
    noReplies: "No reply within the exchange window.",
    sent: "Sent",
    emptyLog: "Frames you send and the replies they get will appear here.",
    clear: "Clear",
  },
};

export type Messages = typeof en;
