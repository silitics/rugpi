import React from "react"

type DeviceType = "generic" | "specific" | "unknown"

type DeviceTier = "1" | "2" | "3"

type Target = "unknown" | "generic-grub-efi" | "rpi-tryboot" | "rpi-uboot"

type Architecture = "amd64" | "arm64" | "armhf"

type DeviceInfo = {
  name: string
  type: DeviceType
  tier: DeviceTier
  target: Target
  architectures: Architecture[]
}

const DEVICES: DeviceInfo[] = [
  {
    name: "Generic (Grub, EFI)",
    type: "generic",
    tier: "1",
    target: "generic-grub-efi",
    architectures: ["arm64", "amd64"],
  },
  {
    name: "Raspberry Pi 5",
    type: "specific",
    tier: "1",
    target: "rpi-tryboot",
    architectures: ["arm64", "armhf"],
  },
  {
    name: "Raspberry Pi 4",
    type: "specific",
    tier: "1",
    target: "rpi-tryboot",
    architectures: ["arm64", "armhf"],
  },
  {
    name: "Raspberry Pi CM4",
    type: "specific",
    tier: "2",
    target: "rpi-tryboot",
    architectures: ["arm64", "armhf"],
  },
  {
    name: "Raspberry Pi 3",
    type: "specific",
    tier: "2",
    target: "rpi-uboot",
    architectures: ["arm64", "armhf"],
  },
  {
    name: "Raspberry Pi Zero 2 W",
    type: "specific",
    tier: "2",
    target: "rpi-uboot",
    architectures: ["arm64", "armhf"],
  },
  {
    name: "Raspberry Pi 2 v1.2",
    type: "specific",
    tier: "3",
    target: "rpi-uboot",
    architectures: ["armhf"],
  },
  {
    name: "Raspberry Pi 2",
    type: "specific",
    tier: "3",
    target: "rpi-uboot",
    architectures: ["armhf"],
  },
  {
    name: "Raspberry Pi 1",
    type: "specific",
    tier: "3",
    target: "rpi-uboot",
    architectures: ["armhf"],
  },
  {
    name: "Raspberry Pi Zero",
    type: "specific",
    tier: "3",
    target: "rpi-uboot",
    architectures: ["armhf"],
  },
  {
    name: "Unknown",
    type: "unknown",
    tier: "3",
    target: "unknown",
    architectures: ["arm64", "amd64", "armhf"],
  },
]

const TIER_EMOJIS: { [tier in DeviceTier]: string } = {
  "1": "🥇",
  "2": "🥈",
  "3": "🥉",
}

const DeviceCard: React.FC<{ info: DeviceInfo }> = ({ info }) => {
  return (
    <div className="w-96 m-4 bg-gray-700 rounded relative">
      <div className="text-2xl absolute right-2 top-2">
        {TIER_EMOJIS[info.tier]}
      </div>
      <div className="p-4">
        <h3>{info.name}</h3>
        <div className="flex space-x-2 text-sm mb-4">
          {info.architectures.map((arch, idx) => (
            <div key={idx} className="bg-white text-black px-1 rounded">
              {arch}
            </div>
          ))}
        </div>
        <code>target = "{info.target}"</code>
      </div>
    </div>
  )
}

const DeviceCards: React.FC<{ type: DeviceType }> = ({ type }) => {
  return (
    <p className="flex flex-wrap">
      {DEVICES.map(
        (device, idx) =>
          device.type === type && <DeviceCard key={idx} info={device} />,
      )}
    </p>
  )
}

export default DeviceCards
