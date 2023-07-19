import React from "react"
import clsx from "clsx"
import styles from "./styles.module.css"

type FeatureItem = {
  title: string
  description: JSX.Element
}

const FeatureList: FeatureItem[] = [
  {
    title: "Reliability Focused",
    description: (
      <>
        Rugpi's focus on reliability ensures uninterrupted operation and
        minimizes costly repairs in the field, making it the ideal platform for
        businesses developing Raspberry Pi-based embedded devices.
      </>
    ),
  },
  {
    title: "Managed State",
    description: (
      <>
        Simplify embedded device development with Rugpi's managed state feature.
        Effortlessly implement factory reset functionality and safeguard against
        accidental state corrupting the system.
      </>
    ),
  },
  {
    title: "Over-the-Air Updates",
    description: (
      <>
        Streamline software updates for embedded devices with Rugpi's
        robust over-the-air update capability. Seamlessly deliver the latest features
        and enhancements while minimizing disruptions.
      </>
    ),
  },
]

function Feature({ title, description }: FeatureItem) {
  return (
    <div className={clsx("col col--4")}>
      <div className="text--center padding-horiz--md">
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  )
}

export default function HomepageFeatures(): JSX.Element {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  )
}
