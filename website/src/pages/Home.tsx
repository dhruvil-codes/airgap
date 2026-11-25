import { Hero } from "../sections/Hero";
import { BentoSection } from "../sections/Bento";
import { BoundarySection, FaqSection, GatesSection, SetupSection } from "../sections/Sections";
import { CtaBanner } from "../sections/CtaBanner";

export function HomePage() {
  return (
    <>
      <Hero />
      <BoundarySection />
      <BentoSection />
      <GatesSection />
      <SetupSection />
      <FaqSection />
      <CtaBanner />
    </>
  );
}
