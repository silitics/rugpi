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
        Rugix's focus on reliability ensures uninterrupted operation and
        minimizes costly repairs in the field, making it the ideal platform for
        businesses developing Linux-based embedded devices.
      </>
    ),
  },
  {
    title: "Over-the-Air Updates",
    description: (
      <>
        Streamline software updates for embedded devices with Rugix's robust and secure
        over-the-air update capability. Seamlessly deliver the latest features
        and enhancements while minimizing disruptions.
      </>
    ),
  },
  {
    title: "Managed State",
    description: (
      <>
        Simplify embedded device development with Rugix's managed state feature.
        Effortlessly implement factory reset functionality and safeguard against
        accidental state corrupting the system.
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
