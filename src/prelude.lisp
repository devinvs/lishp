(defun empty? (a)
  (or (= a '()) (= a "")))

(defun second (x)
  (first (rest x)))

(defun third (x)
  (first (first (rest x))))

(defun nth (x l)
  (if (= x 0)
    (first l)
    (nth (- x 1) (rest l))))

(defun map (l f)
  (if (empty? l)
    '()
    (cons (f (first l)) (map (rest l) f))))

(defun filter (l f)
  (if (empty? l)
    '()
    (if (f (first l))
      (cons (first l) (filter (rest l) f))
      (filter (rest l) f))))

(defun reduce (l f)
  (if (empty? (rest l))
    (first l)
    (f (first l) (reduce (rest l) f))))

(defun contains (l s)
  (if (empty? l)
    false
    (if (= s (first l))
      true
      (contains (rest l) s))))

(defun take (l n)
  (if (= n 0)
    '()
    (cons (first l) (take (rest l) (- n 1)))))

(defun drop (l n)
  (if (= n 0)
    l
    (drop (rest l) (- n 1))))

(defun split-at (l n)
  (list (take l (+ n 1)) (drop l (+ n 1))))

(alias ~ /home/devin)
