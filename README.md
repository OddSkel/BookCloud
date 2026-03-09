# BookCloud


## Setup Pyhton data_clean
First ypu  must have a folder called csvs with all genre csv files

1. Create .venv
python3 -m venv venv

2. Activate .venv
source venv/bin/activate

3. Save installed packages to requirements.txt

pip freeze > requirements.txt

4. Install packages from requirements.txt

pip install -r requirements.txt

5. Close .venv
deactivate




MAX_CHUNKS_PER_FILE=1 CHUNKSIZE=10000 python index.py