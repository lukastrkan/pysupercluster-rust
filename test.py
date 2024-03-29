import pysupercluster
import json
x = pysupercluster.PySupercluster(0, 16, 2, 40, 512, 64)
js = json.load(open('points.json'))
x.load(js)
r = x.get_clusters([-180, -85, 180, 85], 10)
print(r[:10])
