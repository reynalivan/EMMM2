import { motion } from 'motion/react';
import { usePrefersReducedMotion } from './demoTypes';

export default function AnimatedLogo() {
  const prefersReduced = usePrefersReducedMotion();
  const dur = prefersReduced ? 0 : 1.5;

  return (
    <motion.div
      data-testid="logo"
      className="relative w-full h-full flex items-center justify-center cursor-default bg-transparent"
      whileHover="hover"
      initial="initial"
      animate="idle"
      variants={{
        initial: { filter: 'blur(10px)', opacity: 0 },
        idle: { filter: 'blur(0px)', opacity: 1, transition: { duration: dur, ease: 'easeOut' } },
      }}
    >
      <svg
        viewBox="0 0 450 450"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="w-full h-full drop-shadow-md"
      >
        <rect width="450" height="450" rx="122" fill="url(#paint0_radial_0_1)" />

        {/* Top Path */}
        <motion.path
          d="M302.783 63.75H136.5C98.1162 63.75 67 94.8662 67 133.25C67 171.634 98.1162 202.75 136.5 202.75H189.996C208.55 202.75 226.428 195.779 240.086 183.22L370 63.75V171.5"
          stroke="white"
          strokeWidth="34"
          strokeMiterlimit="16"
          strokeLinecap="round"
          strokeLinejoin="round"
          initial={{ pathLength: 0, opacity: 0 }}
          animate={{ pathLength: 1, opacity: 1 }}
          transition={{ duration: dur, ease: 'easeOut' }}
        />

        {/* Bottom Path */}
        <motion.path
          d="M299 247H136.5C98.1162 247 67 278.116 67 316.5C67 354.884 98.1162 386 136.5 386H296C336.869 386 370 352.869 370 312V304"
          stroke="white"
          strokeWidth="34"
          strokeLinecap="round"
          strokeLinejoin="round"
          initial={{ pathLength: 0, opacity: 0 }}
          animate={{ pathLength: 1, opacity: 1 }}
          transition={{ duration: dur, ease: 'easeOut', delay: prefersReduced ? 0 : 0.2 }}
        />

        {/* Top Circle */}
        <motion.circle
          cy="132.5"
          r="40.5"
          fill="white"
          variants={{
            initial: { opacity: 0, scale: 0, cx: 136.5 },
            idle: {
              opacity: 1,
              scale: 1,
              cx: prefersReduced ? 136.5 : 155,
              transition: {
                opacity: { duration: 0.5, delay: prefersReduced ? 0 : 1.0 },
                scale: { duration: 0.5, delay: prefersReduced ? 0 : 1.0 },
                cx: {
                  duration: 2,
                  ease: 'easeInOut',
                  repeat: Infinity,
                  repeatType: 'mirror',
                  repeatDelay: 3,
                  delay: prefersReduced ? 0 : 3.0,
                },
              },
            },
            hover: {
              scale: 0.75,
              transition: { type: 'spring', bounce: 0.5, duration: 0.4 },
            },
          }}
        />

        {/* Bottom Circle */}
        <motion.circle
          cy="316.5"
          r="40.5"
          fill="white"
          variants={{
            initial: { opacity: 0, scale: 0, cx: 300.5 },
            idle: {
              opacity: 1,
              scale: 1,
              cx: prefersReduced ? 300.5 : 210,
              transition: {
                opacity: { duration: 0.5, delay: prefersReduced ? 0 : 1.2 },
                scale: { duration: 0.5, delay: prefersReduced ? 0 : 1.2 },
                cx: {
                  duration: 2,
                  ease: 'easeInOut',
                  repeat: Infinity,
                  repeatType: 'mirror',
                  repeatDelay: 3,
                  delay: prefersReduced ? 0 : 4.0,
                },
              },
            },
            hover: {
              scale: 0.75,
              transition: { type: 'spring', bounce: 0.5, duration: 0.4 },
            },
          }}
        />

        <defs>
          <radialGradient
            id="paint0_radial_0_1"
            cx="0"
            cy="0"
            r="1"
            gradientUnits="userSpaceOnUse"
            gradientTransform="translate(225 225) rotate(90) scale(225)"
          >
            <motion.stop
              offset="0"
              variants={{
                initial: { stopColor: '#64B3E4' },
                idle: {
                  stopColor: prefersReduced ? '#64B3E4' : ['#64B3E4', '#A390E4', '#64B3E4'],
                  transition: { duration: 6, ease: 'easeInOut', repeat: Infinity },
                },
              }}
            />
            <motion.stop
              offset="1"
              variants={{
                initial: { stopColor: '#5B8AFF' },
                idle: {
                  stopColor: prefersReduced ? '#5B8AFF' : ['#5B8AFF', '#8F75F0', '#5B8AFF'],
                  transition: { duration: 6, ease: 'easeInOut', repeat: Infinity, delay: 0.5 },
                },
              }}
            />
          </radialGradient>
        </defs>
      </svg>
    </motion.div>
  );
}
