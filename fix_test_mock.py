import os

path1 = r"src\shared\lib\hooks\bulkToastMessages.test.ts"
with open(path1, "r", encoding="utf-8") as f:
    c = f.read()

c = c.replace("vi.mock('../../shared/i18n/config'", "vi.mock('@/shared/i18n/config'")
c = c.replace("vi.mock('../../shared/lib/appError'", "vi.mock('@/shared/lib/appError'")
c = c.replace("vi.mock('../../shared/lib/disabledPrefix'", "vi.mock('@/shared/lib/disabledPrefix'")

with open(path1, "w", encoding="utf-8") as f:
    f.write(c)
