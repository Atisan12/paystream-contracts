/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // @stellar/stellar-sdk pulls in a few Node built-ins; disable them in the browser bundle.
  webpack: (config) => {
    config.resolve.fallback = { ...config.resolve.fallback, fs: false, net: false, tls: false };
    return config;
  },
};

export default nextConfig;
