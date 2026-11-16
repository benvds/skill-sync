class SkillSync < Formula
  desc "Sync Agent Skills into the directories used by AI coding agents"
  homepage "https://github.com/benvds/skill-sync"
  version "1.0.0"

  on_macos do
    on_arm do
      url "https://github.com/benvds/skill-sync/releases/download/v1.0.0/skill-sync-v1.0.0-aarch64-apple-darwin.tar.gz"
      sha256 "e1254fc3e06ed908571c277654228981e8008c867c8c79763f42c6ba5b6c1431"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/benvds/skill-sync/releases/download/v1.0.0/skill-sync-v1.0.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "6ca80274eee54c294910d18355708146f28851857c1e98268b627053dfbce1ed"
    end
  end

  depends_on "git"

  def install
    bin.install "skill-sync"
  end

  test do
    system "#{bin}/skill-sync", "--version"
  end
end
